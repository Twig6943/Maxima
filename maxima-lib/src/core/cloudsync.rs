use serde::{Deserialize, Serialize};

pub struct CloudSyncClient {
    
}

impl CloudSyncClient {

}

macro_rules! cloudsync_type {
    (
        $(#[$message_attr:meta])*
        $message_name:ident;
        attr {
            $(
                $(#[$attr_field_attr:meta])*
                $attr_field:ident: $attr_field_type:ty
            ),* $(,)?
        },
        data {
            $(
                $(#[$field_attr:meta])*
                $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        paste::paste! {
            // Main struct definition
            $(#[$message_attr])*
            #[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
            #[serde(rename_all = "camelCase")]
            pub struct [<CloudSync $message_name>] {
                $(
                    $(#[$attr_field_attr])*
                    #[serde(rename = "@" $attr_field)]
                    pub [<attr_ $attr_field>]: $attr_field_type,
                )*
                $(
                    $(#[$field_attr])*
                    pub $field: $field_type,
                )*
            }
        }
    }
}

macro_rules! cloudsync_enum {
    ($name:ident, { $($field:tt)* }) => {
        paste::paste! {
            #[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
            #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
            pub enum [<CloudSync $name>] {
                #[default]
                $($field)*
            }
        }
    };
}
