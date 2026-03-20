#[cfg(not(feature = "server"))]
#[macro_export]
macro_rules! define_commands {
    ($($kind:ident($req:ty)),* $(,)?) => {
        define_commands_web!($($kind($req)),*);
    };
}

#[cfg(feature = "server")]
#[macro_export]
macro_rules! define_commands {
    ($($kind:ident($req:ty, $handler:ty)),* $(,)?) => {
        define_commands_web!($($kind($req)),*);
        define_commands_server!($($kind($req, $handler)),*);
    };
}
