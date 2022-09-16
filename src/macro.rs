#[macro_export]
macro_rules! static_unreachable {
    () => {
        #[allow(unreachable_code)]
        {
            let should_not_be_reachable;
            should_not_be_reachable
        }
    };
}