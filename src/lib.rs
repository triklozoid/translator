// Declare and re-export modules
pub mod config;
pub mod settings;
pub mod translation;
pub mod ui;

// Re-export the clone macro for use in tests
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident $(: $t:ty)?),+ => $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            $body
        }
    );
    // This variant clones ONE variable using @strong syntax
    (@strong $n:ident => $body:expr) => (
        {
            let $n = $n.clone();
            $body
        }
    );
     (@weak $n:ident => $body:expr) => (
        {
            let $n = $n.downgrade();
            $body
        }
    );
     (@weak $n:ident $(: $t:ty)? = $e:expr => $body:expr) => (
        {
            let $n = $e.downgrade();
            $body
        }
    );
}
