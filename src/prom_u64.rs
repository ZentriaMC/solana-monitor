use prometheus::core::{AtomicU64, GenericGauge, GenericGaugeVec};

pub type GaugeU64 = GenericGauge<AtomicU64>;
pub type GaugeVecU64 = GenericGaugeVec<AtomicU64>;
