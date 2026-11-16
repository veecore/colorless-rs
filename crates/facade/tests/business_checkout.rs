#![cfg(feature = "relaxed-drop")]

use colorless::{Async, AsyncExt, colorless};
use colorless_core::test_util::block_on_test;
use std::future::poll_fn;
use std::sync::{Arc, Mutex};
use std::task::Poll;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckoutRequest {
    account_id: u64,
    lines: Vec<RequestedLine>,
    coupon: Option<String>,
    expedited: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RequestedLine {
    sku: String,
    quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct Account {
    id: u64,
    active: bool,
    loyalty_percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Quote {
    unit_price: u64,
    available: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PricedLine {
    sku: String,
    quantity: u32,
    unit_price: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PricedCart {
    account_id: u64,
    account: Account,
    lines: Vec<PricedLine>,
    coupon: Option<String>,
    expedited: bool,
    subtotal: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReceiptDraft {
    account_id: u64,
    lines: Vec<PricedLine>,
    subtotal: u64,
    loyalty_discount: u64,
    coupon_discount: u64,
    shipping: u64,
    total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Receipt {
    order_id: u64,
    draft: ReceiptDraft,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CheckoutError {
    EmptyCart,
    InactiveAccount,
    InvalidQuantity(String),
    UnknownSku(String),
    InsufficientStock {
        sku: String,
        requested: u32,
        available: u32,
    },
    InvalidCoupon(String),
    ArithmeticOverflow,
    PersistenceFailed,
}

#[colorless]
trait CheckoutGateway {
    fn load_account(&self, account_id: u64) -> Result<Account, CheckoutError>;

    fn quote(&self, sku: String) -> Result<Quote, CheckoutError>;

    fn reserve(&self, sku: String, quantity: u32) -> Result<(), CheckoutError>;

    fn coupon_percent(&self, account_id: u64, coupon: String) -> Result<u32, CheckoutError>;

    fn expedited_shipping(&self, subtotal: u64) -> Result<u64, CheckoutError>;

    fn persist(&self, draft: ReceiptDraft) -> Result<u64, CheckoutError>;

    #[allow(unused_mut)]
    fn price_cart(&self, request: CheckoutRequest) -> Result<PricedCart, CheckoutError> {
        let CheckoutRequest {
            mut account_id,
            lines,
            mut coupon,
            mut expedited,
        } = request;

        if lines.is_empty() {
            return Err(CheckoutError::EmptyCart);
        }

        let mut account = self.load_account(account_id).resolve?;
        if !account.active {
            return Err(CheckoutError::InactiveAccount);
        }

        let mut subtotal = 0_u64;
        let mut priced_lines = Vec::new();
        let gateway = self;

        #[default_break_state]
        #[colorless::loop_state(
            pub mut account_id,
            pub mut coupon,
            pub mut expedited,
            pub mut account,
            pub mut subtotal,
            pub mut priced_lines,
            gateway
        )]
        for line in lines {
            if line.quantity == 0 {
                return Err(CheckoutError::InvalidQuantity(line.sku));
            }

            let quote = gateway.quote(line.sku.clone()).resolve?;
            if quote.available < line.quantity {
                return Err(CheckoutError::InsufficientStock {
                    sku: line.sku,
                    requested: line.quantity,
                    available: quote.available,
                });
            }

            let line_total = quote
                .unit_price
                .checked_mul(u64::from(line.quantity))
                .ok_or(CheckoutError::ArithmeticOverflow)?;
            subtotal = subtotal
                .checked_add(line_total)
                .ok_or(CheckoutError::ArithmeticOverflow)?;

            gateway.reserve(line.sku.clone(), line.quantity).resolve?;
            priced_lines.push(PricedLine {
                sku: line.sku,
                quantity: line.quantity,
                unit_price: quote.unit_price,
            });
        }

        Ok::<PricedCart, CheckoutError>(PricedCart {
            account_id,
            account,
            lines: priced_lines,
            coupon,
            expedited,
            subtotal,
        })
    }

    fn finish_checkout(&self, cart: PricedCart) -> Result<Receipt, CheckoutError> {
        let PricedCart {
            account_id,
            account,
            lines,
            coupon,
            expedited,
            subtotal,
        } = cart;

        let loyalty_discount = subtotal
            .checked_mul(u64::from(account.loyalty_percent))
            .ok_or(CheckoutError::ArithmeticOverflow)?
            / 100;

        let coupon_discount = if let Some(code) = coupon {
            // This intentionally shadows the outer numeric account id. The
            // continuation still needs the outer binding for the receipt.
            let account_id = code;
            let percent = self.coupon_percent(account.id, account_id).resolve?;
            subtotal
                .checked_mul(u64::from(percent))
                .ok_or(CheckoutError::ArithmeticOverflow)?
                / 100
        } else {
            0
        };

        let shipping = if expedited {
            self.expedited_shipping(subtotal).resolve?
        } else {
            0
        };

        let total = subtotal
            .checked_sub(loyalty_discount)
            .and_then(|value| value.checked_sub(coupon_discount))
            .and_then(|value| value.checked_add(shipping))
            .ok_or(CheckoutError::ArithmeticOverflow)?;

        let draft = ReceiptDraft {
            account_id,
            lines,
            subtotal,
            loyalty_discount,
            coupon_discount,
            shipping,
            total,
        };
        let order_id = self.persist(draft.clone()).resolve?;

        Ok::<Receipt, CheckoutError>(Receipt { order_id, draft })
    }

    fn checkout(&self, request: CheckoutRequest) -> Result<Receipt, CheckoutError> {
        let cart = self.price_cart(request).resolve?;
        self.finish_checkout(cart).resolve
    }
}

#[derive(Clone)]
struct AsyncGateway {
    trace: Arc<Mutex<Vec<String>>>,
    fail_persistence: bool,
}

impl AsyncGateway {
    fn new(fail_persistence: bool) -> Self {
        Self {
            trace: Arc::new(Mutex::new(Vec::new())),
            fail_persistence,
        }
    }

    fn trace(&self) -> Vec<String> {
        self.trace.lock().unwrap().clone()
    }

    async fn operation<T>(&self, name: String, result: T) -> T {
        self.trace.lock().unwrap().push(format!("{name}:start"));
        yield_once().await;
        self.trace.lock().unwrap().push(format!("{name}:end"));
        result
    }

    async fn load_account_native(&self, account_id: u64) -> Result<Account, CheckoutError> {
        let account = Account {
            id: account_id,
            active: account_id != 0,
            loyalty_percent: 5,
        };
        self.operation(format!("account:{account_id}"), Ok(account))
            .await
    }

    async fn quote_native(&self, sku: String) -> Result<Quote, CheckoutError> {
        let result = match sku.as_str() {
            "book" => Ok(Quote {
                unit_price: 1_200,
                available: 5,
            }),
            "pen" => Ok(Quote {
                unit_price: 250,
                available: 20,
            }),
            _ => Err(CheckoutError::UnknownSku(sku.clone())),
        };
        self.operation(format!("quote:{sku}"), result).await
    }

    async fn reserve_native(&self, sku: String, quantity: u32) -> Result<(), CheckoutError> {
        self.operation(format!("reserve:{sku}:{quantity}"), Ok(()))
            .await
    }

    async fn coupon_percent_native(
        &self,
        account_id: u64,
        coupon: String,
    ) -> Result<u32, CheckoutError> {
        let result = if coupon == "SAVE10" {
            Ok(10)
        } else {
            Err(CheckoutError::InvalidCoupon(coupon.clone()))
        };
        self.operation(format!("coupon:{account_id}:{coupon}"), result)
            .await
    }

    async fn expedited_shipping_native(&self, subtotal: u64) -> Result<u64, CheckoutError> {
        self.operation(format!("shipping:{subtotal}"), Ok(500))
            .await
    }

    async fn persist_native(&self, draft: ReceiptDraft) -> Result<u64, CheckoutError> {
        let result = if self.fail_persistence {
            Err(CheckoutError::PersistenceFailed)
        } else {
            Ok(9_001)
        };
        self.operation(format!("persist:{}", draft.total), result)
            .await
    }
}

#[colorless(color = Async)]
impl CheckoutGateway for AsyncGateway {
    async fn load_account(&self, account_id: u64) -> Result<Account, CheckoutError> {
        self.load_account_native(account_id).await
    }

    async fn quote(&self, sku: String) -> Result<Quote, CheckoutError> {
        self.quote_native(sku).await
    }

    async fn reserve(&self, sku: String, quantity: u32) -> Result<(), CheckoutError> {
        self.reserve_native(sku, quantity).await
    }

    async fn coupon_percent(&self, account_id: u64, coupon: String) -> Result<u32, CheckoutError> {
        self.coupon_percent_native(account_id, coupon).await
    }

    async fn expedited_shipping(&self, subtotal: u64) -> Result<u64, CheckoutError> {
        self.expedited_shipping_native(subtotal).await
    }

    async fn persist(&self, draft: ReceiptDraft) -> Result<u64, CheckoutError> {
        self.persist_native(draft).await
    }
}

async fn checkout_reference(
    gateway: &AsyncGateway,
    request: CheckoutRequest,
) -> Result<Receipt, CheckoutError> {
    let CheckoutRequest {
        account_id,
        lines,
        coupon,
        expedited,
    } = request;

    if lines.is_empty() {
        return Err(CheckoutError::EmptyCart);
    }

    let account = gateway.load_account_native(account_id).await?;
    if !account.active {
        return Err(CheckoutError::InactiveAccount);
    }

    let mut subtotal = 0_u64;
    let mut priced_lines = Vec::new();

    for line in lines {
        if line.quantity == 0 {
            return Err(CheckoutError::InvalidQuantity(line.sku));
        }

        let quote = gateway.quote_native(line.sku.clone()).await?;
        if quote.available < line.quantity {
            return Err(CheckoutError::InsufficientStock {
                sku: line.sku,
                requested: line.quantity,
                available: quote.available,
            });
        }

        let line_total = quote
            .unit_price
            .checked_mul(u64::from(line.quantity))
            .ok_or(CheckoutError::ArithmeticOverflow)?;
        subtotal = subtotal
            .checked_add(line_total)
            .ok_or(CheckoutError::ArithmeticOverflow)?;

        gateway
            .reserve_native(line.sku.clone(), line.quantity)
            .await?;
        priced_lines.push(PricedLine {
            sku: line.sku,
            quantity: line.quantity,
            unit_price: quote.unit_price,
        });
    }

    let loyalty_discount = subtotal
        .checked_mul(u64::from(account.loyalty_percent))
        .ok_or(CheckoutError::ArithmeticOverflow)?
        / 100;
    let coupon_discount = if let Some(code) = coupon {
        let percent = gateway.coupon_percent_native(account.id, code).await?;
        subtotal
            .checked_mul(u64::from(percent))
            .ok_or(CheckoutError::ArithmeticOverflow)?
            / 100
    } else {
        0
    };
    let shipping = if expedited {
        gateway.expedited_shipping_native(subtotal).await?
    } else {
        0
    };
    let total = subtotal
        .checked_sub(loyalty_discount)
        .and_then(|value| value.checked_sub(coupon_discount))
        .and_then(|value| value.checked_add(shipping))
        .ok_or(CheckoutError::ArithmeticOverflow)?;
    let draft = ReceiptDraft {
        account_id,
        lines: priced_lines,
        subtotal,
        loyalty_discount,
        coupon_discount,
        shipping,
        total,
    };
    let order_id = gateway.persist_native(draft.clone()).await?;
    Ok(Receipt { order_id, draft })
}

async fn yield_once() {
    let mut yielded = false;
    poll_fn(move |context| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

fn successful_request() -> CheckoutRequest {
    CheckoutRequest {
        account_id: 7,
        lines: vec![
            RequestedLine {
                sku: String::from("book"),
                quantity: 2,
            },
            RequestedLine {
                sku: String::from("pen"),
                quantity: 3,
            },
        ],
        coupon: Some(String::from("SAVE10")),
        expedited: true,
    }
}

#[test]
fn colorless_checkout_matches_native_async_business_logic() {
    let colorless_gateway = AsyncGateway::new(false);
    let reference_gateway = AsyncGateway::new(false);
    let request = successful_request();

    let computation = CheckoutGateway::<Async>::checkout(&colorless_gateway, request.clone());
    assert!(colorless_gateway.trace().is_empty());
    let native = checkout_reference(&reference_gateway, request);

    eprintln!("Colorless: {}", size_of_val(&computation));
    eprintln!("Native: {}", size_of_val(&native));

    let colorless_result = block_on_test(computation.async_());
    let reference_result = block_on_test(native);

    assert_eq!(colorless_result, reference_result);
    assert_eq!(colorless_gateway.trace(), reference_gateway.trace());
}
