use prost::Message;
use prost_reflect::{DynamicMessage, MessageDescriptor};
use tonic::{
    Status,
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
};

/// A codec that bridges `serde_json::Value` and Protobuf binary format.
pub struct JsonCodec {
    req_desc: MessageDescriptor,
    res_desc: MessageDescriptor,
}

impl JsonCodec {
    pub fn new(req_desc: MessageDescriptor, res_desc: MessageDescriptor) -> Self {
        Self { req_desc, res_desc }
    }
}

impl Codec for JsonCodec {
    type Encode = serde_json::Value;
    type Decode = serde_json::Value;

    type Encoder = JsonEncoder;
    type Decoder = JsonDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        JsonEncoder(self.req_desc.clone())
    }

    fn decoder(&mut self) -> Self::Decoder {
        JsonDecoder(self.res_desc.clone())
    }
}

pub struct JsonEncoder(MessageDescriptor);

impl Encoder for JsonEncoder {
    type Item = serde_json::Value;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        let msg = DynamicMessage::deserialize(self.0.clone(), item).map_err(|e| {
            Status::invalid_argument(format!(
                "JSON structure does not match Protobuf schema: {}",
                e
            ))
        })?;

        msg.encode_raw(dst);
        Ok(())
    }
}

pub struct JsonDecoder(MessageDescriptor);

impl Decoder for JsonDecoder {
    type Item = serde_json::Value;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let mut msg = DynamicMessage::new(self.0.clone());
        msg.merge(src)
            .map_err(|e| Status::internal(format!("Failed to decode Protobuf bytes: {}", e)))?;

        let value = serde_json::to_value(&msg)
            .map_err(|e| Status::internal(format!("Failed to map response to JSON: {}", e)))?;

        Ok(Some(value))
    }
}
