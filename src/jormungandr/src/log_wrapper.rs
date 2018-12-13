pub use log::log;

/// Logs a message at the debug level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// debug!("Error: {} on port {}", err_info, port);
/// debug!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!debug  {
    ($($arg:tt)*) => { log!(level: log::Level::Debug, $($arg)*)};
}

/// Logs a message at the error level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// error!("Error: {} on port {}", err_info, port);
/// error!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!error {
    ($($arg:tt)*) => { log!(level: log::Level::Error, $($arg)*)};
}

/// Logs a message at the info level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// info!("Error: {} on port {}", err_info, port);
/// info!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!info{
    ($($arg:tt)*) => { log!(level: log::Level::Info, $($arg)*)};
}

/// Logs a message at the trace level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// trace!("Error: {} on port {}", err_info, port);
/// trace!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!trace {
    ($($arg:tt)*) => { log!(level: log::Level::Trace, $($arg)*)};
}

/// Logs a message at the warn level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// warn!("Error: {} on port {}", err_info, port);
/// warn!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!warn {
    ($($arg:tt)*) => { log!(level: log::Level::Warn, $($arg)*)};
}

/// Standard logging macros.
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// log!(level: log::Level::Error, "Error: {} on port {}", err_info, port;
///   user = "Loki"
///   location = "Fólkvangr"
///   );
/// ```
#[macro_export]
macro_rules!log {
    (level: $lvl:expr, $msg:expr ; $($name:ident = $val:expr),+ ) =>
        { log::log!($lvl, concat!(concat!($("[",stringify!($name),"= {:#?} ]",)+),$msg),$($val),+ ) };
    (level: $lvl:expr, $msg:expr, $($params:expr),+ ; $($name:ident = $val:expr),+ ) =>
        { log::log!($lvl, concat!(concat!($("[",stringify!($name),"=","{:?}","]",)+),$msg), $($val,)+ $($params,)* ) };
    (level: $lvl:expr, $msg:expr, $($params:expr),+) =>
        { log::log!($lvl, $msg, $( $params,)+ ) };
    (level: $lvl:expr, $msg:expr, $($params:expr),+ , ) =>
        { log::log!($lvl, $msg, $( $params,)+ ) };
    (level: $lvl:expr, $msg:expr) =>
        { log::log!($lvl, $msg ) };

}
