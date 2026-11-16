#[macro_use]
mod helper;

macro_rules! declare_colors {
    ($( $(#[$m:meta])? $name:ident ),*) => {
        $(
            $(#[$m])?
            mod $name;
        )*

        $(
            $(#[$m])?
            pub use $name::*;
        )*
    };
}

declare_colors! {
    fallible,
    #[cfg(feature = "rainbow")]
    fallible_option,
    identity,
    optional,
    #[cfg(feature = "rainbow")]
    optional_result
}
