pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub use {builder::Builder, collector::Collector};

mod builder;
mod collector;
mod emf;
#[cfg(test)]
mod test;
