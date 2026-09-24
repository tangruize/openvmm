// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl View for SavedStateBlob {
    type V = mesh::payload::message::ProtobufAnyView;

    open spec fn view(&self) -> Self::V {
        self.0@
    }
}

}
