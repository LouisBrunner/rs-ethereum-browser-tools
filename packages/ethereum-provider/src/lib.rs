#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod errors;
pub mod provider;

#[cfg(feature = "yew")]
pub mod yew;
