// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

pub struct ProtobufAnyView {
    pub type_url: Seq<char>,
    pub bytes: Seq<u8>,
}

impl View for ProtobufMessage {
    type V = Seq<u8>;

    open spec fn view(&self) -> Seq<u8> {
        self.0@
    }
}

impl View for ProtobufAny {
    type V = ProtobufAnyView;

    open spec fn view(&self) -> ProtobufAnyView {
        ProtobufAnyView { type_url: self.type_url@, bytes: self.value@ }
    }
}

}
