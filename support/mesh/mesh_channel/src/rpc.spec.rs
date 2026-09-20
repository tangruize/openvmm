// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

mod std_error_spec {
    use core::fmt::{Debug, Display};
    use vstd::prelude::*;

    verus! {

    #[verifier::external_trait_specification]
    pub trait ExCoreError: Debug + Display {
        type ExternalTraitSpecificationFor: core::error::Error;

        fn source(&self) -> Option<&(dyn core::error::Error + 'static)>;
    }

    }
}
