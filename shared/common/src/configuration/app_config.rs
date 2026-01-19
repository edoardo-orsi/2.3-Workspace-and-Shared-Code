use crate::configuration::config::BaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig<T> {
    pub base: BaseConfig,
    pub spec: T,
}
