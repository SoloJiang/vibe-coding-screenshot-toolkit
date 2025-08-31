//! Node.js N-API 绑定（可选 feature: `node`）。
//! 默认不启用，以避免未安装 node toolchain 的链接错误。

#[cfg(feature = "node")]
mod node_api {
    // 直接使用宏定义所在 crate，避免由于关闭 napi 默认特性导致的 re-export 丢失
    #[napi_derive::napi]
    #[allow(dead_code)]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// 当未启用 feature 时，提供同名纯 Rust 函数，便于共享代码调用。
#[cfg(not(feature = "node"))]
#[allow(dead_code)]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
