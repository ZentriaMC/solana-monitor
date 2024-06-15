use prometheus::core::{AtomicU64, GenericGauge, GenericGaugeVec};

pub type GaugeU64 = GenericGauge<AtomicU64>;
pub type GaugeVecU64 = GenericGaugeVec<AtomicU64>;

#[macro_export]
macro_rules! register_gauge_u64 {
    ($OPTS:expr $(,)?) => {{
        $crate::__register_gauge_u64!(GaugeU64, $OPTS)
    }};

    ($NAME:expr, $HELP:expr $(,)?) => {{
        $crate::register_gauge_u64!(opts!($NAME, $HELP))
    }};
}

#[macro_export]
macro_rules! __register_gauge_u64 {
    ($TYPE:ident, $OPTS:expr) => {{
        $crate::__register_gauge_u64!($TYPE, $OPTS, prometheus::default_registry())
    }};

    ($TYPE:ident, $OPTS:expr, $REGISTRY:expr) => {{
        let gauge = $TYPE::with_opts($OPTS).unwrap();
        $REGISTRY.register(Box::new(gauge.clone())).map(|()| gauge)
    }};
}

#[macro_export]
macro_rules! register_gauge_vec_u64 {
    ($OPTS:expr, $LABELS_NAMES:expr $(,)?) => {{
        $crate::__register_gauge_vec_u64!(GaugeVecU64, $OPTS, $LABELS_NAMES)
    }};

    ($NAME:expr, $HELP:expr, $LABELS_NAMES:expr $(,)?) => {{
        $crate::register_gauge_vec_u64!(opts!($NAME, $HELP), $LABELS_NAMES)
    }};
}

#[macro_export]
macro_rules! __register_gauge_vec_u64 {
    ($TYPE:ident, $OPTS:expr, $LABELS_NAMES:expr $(,)?) => {{
        $crate::__register_gauge_vec_u64!(
            $TYPE,
            $OPTS,
            $LABELS_NAMES,
            prometheus::default_registry()
        )
    }};

    ($TYPE:ident, $OPTS:expr, $LABELS_NAMES:expr, $REGISTRY:expr $(,)?) => {{
        let gauge_vec = $TYPE::new($OPTS, $LABELS_NAMES).unwrap();
        $REGISTRY
            .register(Box::new(gauge_vec.clone()))
            .map(|()| gauge_vec)
    }};
}
