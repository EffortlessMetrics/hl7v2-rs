use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum PublishSurface {
    /// Primary Rust API/operator crates.
    Primary,
    /// Binding backend crates for foreign-language packages.
    Bindings,
    /// Primary Rust product crates plus publishable binding backend crates.
    AllPublishable,
}

impl PublishSurface {
    pub(super) fn publish_plan_heading(self) -> &'static str {
        match self {
            Self::Primary => "Primary Rust product crates.io publish order",
            Self::Bindings => "Binding backend crates.io publish order",
            Self::AllPublishable => "All publishable crates.io publish order",
        }
    }

    pub(super) fn dry_run_label(self) -> &'static str {
        match self {
            Self::Primary => "primary Rust product crates.io publish",
            Self::Bindings => "binding backend crates.io package",
            Self::AllPublishable => "all publishable crates.io package",
        }
    }
}
