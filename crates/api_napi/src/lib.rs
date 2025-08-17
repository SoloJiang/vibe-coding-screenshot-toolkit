//! Node.js N-API 绑定（可选 feature: `node`）。
//! 默认不启用，以避免未安装 node toolchain 的链接错误。

#[cfg(feature = "node")]
mod node_api {
    use napi::bindgen_prelude::*;

    #[napi]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// 当未启用 feature 时，提供同名纯 Rust 函数，便于共享代码调用。
#[cfg(not(feature = "node"))]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
