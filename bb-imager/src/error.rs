//! Command error type for this library

use thiserror::Error;

use crate::flasher::{bcf, msp430, sd};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("BeagleConnect Freedom Error: {0}")]
    BeagleConnectFreedomError(#[from] bcf::Error),
    #[error("MSP430 Error: {0}")]
    MSP430Error(#[from] msp430::Error),
    #[error("Io Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Image Error: {0}")]
    ImageError(#[from] crate::img::Error),
    #[error("Sd Card Error: {0}")]
    SdCardError(#[from] sd::Error),
    #[error("{0}")]
    CommanError(#[from] crate::common::Error),
    #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
    #[error("Pb2 MSPM0 Error: {0}")]
    Pb2Error(#[from] crate::flasher::pb2_mspm0::Error),
}
