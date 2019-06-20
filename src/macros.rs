macro_rules! command {
    ($cmd:ident($_self:ident, $c:ident) $b:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec $_self $c $b);
        }
    };
    ($cmd:ident($_self:ident, $c:ident, $m:ident) $b:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec2 $_self $c $m $b);
        }
    };
    ($cmd:ident($_self:ident, $c:ident, $m:ident, $a:ident) $b:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec3 $_self $c $m $a $b);
        }
    };
    ($cmd:ident($_self:ident, $c:ident) $b:block options($o:ident) $opts:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec $_self $c $b);

            command!(__impl_opts $o $opts);
        }
    };
    ($cmd:ident($_self:ident, $c:ident, $m:ident) $b:block options($o:ident) $opts:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec2 $_self $c $m $b);

            command!(__impl_opts $o $opts);
        }
    };
    ($cmd:ident($_self:ident, $c:ident, $m:ident, $a:ident) $b:block options($o:ident) $opts:block) => {
        command!(__impl $cmd);

        impl serenity::framework::standard::Command for $cmd {
            command!(__impl_exec3 $_self $c $m $a $b);

            command!(__impl_opts $o $opts);
        }
    };
    (__impl $cmd:ident) => {
        pub struct $cmd {
            modio: modio::Modio,
            executor: tokio::runtime::TaskExecutor,
        }

        impl $cmd {
            pub fn new(modio: modio::Modio, executor: tokio::runtime::TaskExecutor) -> Self {
                Self { modio, executor }
            }
        }
    };
    (__impl_exec $_self:ident $c:ident $b:block) => {
         fn execute(&$_self, $c: &mut serenity::client::Context,
                    _: &serenity::model::channel::Message,
                    _: serenity::framework::standard::Args)
                    -> std::result::Result<(), serenity::framework::standard::CommandError> {
            $b

            Ok(())
         }
    };
    (__impl_exec2 $_self:ident $c:ident $m:ident $b:block) => {
         fn execute(&$_self, $c: &mut serenity::client::Context,
                    $m: &serenity::model::channel::Message,
                     _: serenity::framework::standard::Args)
                    -> std::result::Result<(), serenity::framework::standard::CommandError> {
            $b

            Ok(())
         }
    };
    (__impl_exec3 $_self:ident $c:ident $m:ident $a:ident $b:block) => {
         fn execute(&$_self, $c: &mut serenity::client::Context,
                    $m: &serenity::model::channel::Message,
                    mut $a: serenity::framework::standard::Args)
                    -> std::result::Result<(), serenity::framework::standard::CommandError> {
            $b

            Ok(())
         }
    };
    (__impl_opts $n:ident $b:block) => {
        fn options(&self) -> std::sync::Arc<serenity::framework::standard::CommandOptions> {
            let mut $n = serenity::framework::standard::CommandOptions::default();
            $b
            std::sync::Arc::new($n)
        }
    };
}
