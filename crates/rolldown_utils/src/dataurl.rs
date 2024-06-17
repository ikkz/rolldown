use std::fmt::Write;
use std::str;

// https://github.com/evanw/esbuild/blob/67cbf87a4909d87a902ca8c3b69ab5330defab0a/internal/helpers/dataurl.go

// Returns the shorter of either a base64-encoded or percent-escaped data URL
pub fn encode_as_shortest_dataurl(mime: &str, buf: &[u8]) -> String {
  let base64 = crate::base64::to_standard_base64(buf);
  let base64_url = format!("data:{mime};base64,{base64}");

  match encode_as_percent_escaped_dataurl(mime, buf) {
    Some(percent_url) if percent_url.len() < base64_url.len() => percent_url,
    _ => base64_url,
  }
}

// See "https://github.com/evanw/esbuild/blob/67cbf87a4909d87a902ca8c3b69ab5330defab0a/scripts/dataurl-escapes.html" for how this was derived
fn encode_as_percent_escaped_dataurl(mime: &str, buf: &[u8]) -> Option<String> {
  if let Ok(text) = str::from_utf8(buf) {
    let mut url = String::with_capacity(text.len() * 3);
    write!(url, "data:{mime},").unwrap();
    let chars = text.chars().collect::<Vec<_>>();
    let mut trailing_start = chars.len();
    while trailing_start > 0 {
      let c = chars[trailing_start - 1];
      if c > 0x20 as char || matches!(c, '\t' | '\n' | '\r') {
        break;
      }
      trailing_start -= 1;
    }
    for (i, &c) in chars.iter().enumerate() {
      if matches!(c, '\t' | '\n' | '\r' | '#')
        || i >= trailing_start
        || (c == '%'
          && i + 2 < chars.len()
          && chars[i + 1].is_ascii_hexdigit()
          && chars[i + 2].is_ascii_hexdigit())
      {
        write!(url, "%{:02X}", c as u32).unwrap();
      } else {
        url.push(c);
      }
    }
    Some(url)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn check(raw: &str, expected: &str) {
    let result = encode_as_percent_escaped_dataurl("text/plain", raw.as_bytes());
    assert!(result.is_some(), "Failed to encode {raw:?}");
    assert_eq!(result.unwrap(), expected, "Test failed for input {raw:?}");
  }

  #[test]
  fn test_encode_data_url() {
    for i in 0..=0x7F {
      let always_escape = i == b'\t' || i == b'\r' || i == b'\n' || i == b'#';
      let trailing_escape = i <= 0x20 || i == b'#';

      let char_str = String::from_utf8(vec![i]).unwrap();

      if trailing_escape {
        check(&char_str, &format!("data:text/plain,%{i:02X}"));
        check(&format!("foo{char_str}"), &format!("data:text/plain,foo%{i:02X}"));
      } else {
        check(&char_str, &format!("data:text/plain,{char_str}"));
        check(&format!("foo{char_str}"), &format!("data:text/plain,foo{char_str}"));
      }

      if always_escape {
        check(&format!("{char_str}foo"), &format!("data:text/plain,%{i:02X}foo"));
      } else {
        check(&format!("{char_str}foo"), &format!("data:text/plain,{char_str}foo"));
      }
    }

    // Test leading vs. trailing
    check(" \t ", "data:text/plain, %09%20");
    check(" \n ", "data:text/plain, %0A%20");
    check(" \r ", "data:text/plain, %0D%20");
    check(" # ", "data:text/plain, %23%20");
    check("\x08#\x08", "data:text/plain,\x08%23%08");

    // Only "%" symbols that could form an escape need to be escaped
    check("%, %3, %33, %333", "data:text/plain,%, %3, %2533, %25333");
  }

  #[test]
  fn test_shortest() {
    assert_eq!(
      encode_as_shortest_dataurl("text/plain", "\n\n\n\n\n".as_bytes()),
      "data:text/plain;base64,CgoKCgo="
    );
    assert_eq!(
      encode_as_shortest_dataurl("text/plain", "\n\n\n".as_bytes()),
      "data:text/plain,%0A%0A%0A"
    );
  }
}
