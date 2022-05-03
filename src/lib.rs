//! Provides an implementation of the [`io::Provider`](crate::provider::io::Provider)
//! using the [`Tokio runtime`](https://docs.rs/tokio/latest/tokio/runtime/index.html)

mod clock;
mod io;

use s2n_quic_core::{endpoint::Endpoint, inet::SocketAddress};

pub use crate::io::{Builder, Io as Provider, PathHandle};

impl s2n_quic::provider::io::Provider for Provider {
    type PathHandle = PathHandle;
    type Error = std::io::Error;

    fn start<E: Endpoint<PathHandle = Self::PathHandle>>(
        self,
        endpoint: E,
    ) -> Result<SocketAddress, Self::Error> {
        let (_join_handle, local_addr) = Provider::start(self, endpoint)?;
        Ok(local_addr)
    }
}
