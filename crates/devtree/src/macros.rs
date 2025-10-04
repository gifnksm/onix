macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return Err($err.into());
        }
    };
}

macro_rules! bail {
    ($err:expr $(,)?) => {{
        return Err($err.into());
    }};
}

macro_rules! define_error {
    (
        $(#[$attrs:meta])*
        $vis:vis struct $err_ty:ident {
            kind: $err_kind:ty $(,)?
        }
    ) => {
        $(#[$attrs])*
        #[derive(Debug)]
        $vis struct $err_ty {
            #[cfg(feature = "error-with-location")]
            location: &'static ::core::panic::Location<'static>,
            kind: $err_kind,
        }

        impl $err_ty {
            /// Creates a new error from a known kind of error.
            #[track_caller]
            #[must_use]
            $vis fn new(kind: $err_kind) -> Self {
                Self {
                    kind,
                    #[cfg(feature = "error-with-location")]
                    location: ::core::panic::Location::caller(),
                }
            }

            /// Returns the kind of this error.
            #[must_use]
            $vis fn kind(&self) -> &$err_kind {
                &self.kind
            }

            /// Returns the location where this error was created.
            #[must_use]
            #[cfg(feature = "error-with-location")]
            $vis fn location(&self) -> &'static ::core::panic::Location<'static> {
                self.location
            }
        }

        impl ::core::fmt::Display for $err_ty {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.kind, f)
            }
        }

        impl From<$err_kind> for $err_ty {
            #[track_caller]
            fn from(kind: $err_kind) -> Self {
                Self::new(kind)
            }
        }

        impl ::core::error::Error for $err_ty {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                self.kind.source()
            }

            #[cfg(feature = "unstable-provider-api")]
            fn provide<'a>(&'a self, request: &mut ::core::error::Request<'a>) {
                #[cfg(feature = "error-with-location")]
                request.provide_ref(self.location());
            }
        }
    };
}
