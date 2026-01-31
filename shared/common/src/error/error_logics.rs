use std::error::Error;

pub trait ErrorLogic {
    fn report_chain(&self) -> Vec<String>;
}

impl<T: Error> ErrorLogic for T {
    fn report_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut curr: Option<&dyn std::error::Error> = Some(self);

        while let Some(source) = curr {
            chain.push(source.to_string());
            curr = source.source();
        }
        chain
    }
}
