mod blossom;

use std::path::Path;

use anyhow::Result;

fn main() -> Result<()> {
    let mut b = blossom::Blossom::new();
    b.add_descriptor(
        Path::new("/home/elia/code/proto/ono/logistics/server/ono_logistics_server.desc")
    )?;
    Ok(())
}
