use chrono::Local;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// 固定密钥（务必与客户脚本保持一致）
const SECRET: &[u8] = b"MD_GENERATOR_SECRET_2026";

// 后门固定密码（可任意修改）
const BACKDOOR_PASSWORD: &str = "admin123";

/// 每 15 分钟生成一个 6 位动态密码
pub fn current_password() -> String {
    let timestamp = Local::now().timestamp();
    let counter = timestamp / (15 * 60);

    let mut mac = HmacSha256::new_from_slice(SECRET).expect("HMAC can take key of any size");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let num = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);
    format!("{:06}", num % 1_000_000)
}

/// 验证密码：接受后门固定密码或当前动态密码
pub fn verify_password(input: &str) -> bool {
    input == BACKDOOR_PASSWORD || input == current_password()
}
