use anyhow::Result;
use std::io::Write;

use crate::CompressionMethod;

pub fn compress<S: Write>(
    compression: CompressionMethod,
    input: &[u8],
    mut output: S,
) -> Result<()> {
    match compression {
        CompressionMethod::Zstd => {
            let buf = zstd::stream::encode_all(input, 0)?;
            output.write_all(&buf)?;
        }
        CompressionMethod::Oodle => {
            todo!()
            //let buffer = oodle_loader::oodle()?.compress(
            //    input,
            //    oodle_loader::Compressor::Mermaid,
            //    oodle_loader::CompressionLevel::Normal,
            //)?;
            //output.write_all(&buffer)?;
        }
        CompressionMethod::Brotli => {
            todo!()
        }
    }
    Ok(())
}

pub fn decompress(compression: CompressionMethod, input: &[u8], output: &mut [u8]) -> Result<()> {
    match compression {
        CompressionMethod::Zstd => {
            zstd::bulk::decompress_to_buffer(input, output)?;
        }
        CompressionMethod::Oodle => {
            todo!()
            //let status = oodle_loader::oodle()?.decompress(input, output);
            //if status < 0 || status as usize != output.len() {
            //    bail!(
            //        "Oodle decompression failed: expected {} output bytes, got {}",
            //        output.len(),
            //        status,
            //    );
            //}
        }
        CompressionMethod::Brotli => {
            todo!()
        }
    }
    Ok(())
}
