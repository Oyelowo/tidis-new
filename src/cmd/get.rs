use crate::tikv::errors::AsyncResult;
use crate::{Connection, Frame, Parse};
use crate::tikv::string::StringCommandCtx;
use tracing::{debug, instrument};

use crate::config::{is_use_txn_api};

/// Get the value of key.
///
/// If the key does not exist the special value nil is returned. An error is
/// returned if the value stored at key is not a string, because GET only
/// handles string values.
#[derive(Debug)]
pub struct Get {
    /// Name of the key to get
    key: String,
}

impl Get {
    /// Create a new `Get` command which fetches `key`.
    pub fn new(key: impl ToString) -> Get {
        Get {
            key: key.to_string(),
        }
    }

    /// Get the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Parse a `Get` instance from a received frame.
    ///
    /// The `Parse` argument provides a cursor-like API to read fields from the
    /// `Frame`. At this point, the entire frame has already been received from
    /// the socket.
    ///
    /// The `GET` string has already been consumed.
    ///
    /// # Returns
    ///
    /// Returns the `Get` value on success. If the frame is malformed, `Err` is
    /// returned.
    ///
    /// # Format
    ///
    /// Expects an array frame containing two entries.
    ///
    /// ```text
    /// GET key
    /// ```
    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<Get> {
        // The `GET` string has already been consumed. The next value is the
        // name of the key to get. If the next value is not a string or the
        // input is fully consumed, then an error is returned.
        let key = parse.next_string()?;

        Ok(Get { key })
    }

    /// Apply the `Get` command to the specified `Db` instance.
    ///
    /// The response is written to `dst`. This is called by the server in order
    /// to execute a received command.
    #[instrument(skip(self, dst))]
    pub(crate) async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        // Get the value from the shared database state
        let response = match self.get(&self.key).await {
            Ok(val) => val,
            Err(e) => Frame::Error(e.to_string()),
        };

        debug!(?response);

        // Write the response back to the client
        dst.write_frame(&response).await?;

        Ok(())
    }

    async fn get(&self, key: &str) ->AsyncResult<Frame> {
        if is_use_txn_api() {
            StringCommandCtx::new(None).do_async_txnkv_get(key).await
        } else {
            StringCommandCtx::new(None).do_async_rawkv_get(key).await
        }
    }
}
