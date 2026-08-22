//! 抓取层：通用处理（`generic`）+ 站点适配器（`adapters`）。
//!
//! 分工：`generic` 提供不依赖网络的 HTML 处理能力（容器提取/清洗/图片归一化/CF 检测），
//! `adapters` 承载站点专项抓取；上层（`crate::feed`）负责网络 IO 与两条路径的编排。

pub mod adapters;
pub mod generic;
