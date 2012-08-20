#[deny(non_camel_case_types)];

// Rust JSON serialization library
// Copyright (c) 2011 Google Inc.

//! json serialization

import result::{result, ok, err};
import io;
import io::WriterUtil;
import map;
import map::hashmap;
import map::map;

export Json;
export Error;
export to_writer;
export to_str;
export from_reader;
export from_str;
export eq;
export ToJson;

export Num;
export String;
export Boolean;
export List;
export Dict;
export Null;

/// Represents a json value
enum Json {
    Num(float),
    String(@~str),
    Boolean(bool),
    List(@~[Json]),
    Dict(map::hashmap<~str, Json>),
    Null,
}

type Error = {
    line: uint,
    col: uint,
    msg: @~str,
};

/// Serializes a json value into a io::writer
fn to_writer(wr: io::Writer, j: Json) {
    match j {
      Num(n) => wr.write_str(float::to_str(n, 6u)),
      String(s) => wr.write_str(escape_str(*s)),
      Boolean(b) => wr.write_str(if b { ~"true" } else { ~"false" }),
      List(v) => {
        wr.write_char('[');
        let mut first = true;
        for (*v).each |item| {
            if !first {
                wr.write_str(~", ");
            }
            first = false;
            to_writer(wr, item);
        };
        wr.write_char(']');
      }
      Dict(d) => {
        if d.size() == 0u {
            wr.write_str(~"{}");
            return;
        }

        wr.write_str(~"{ ");
        let mut first = true;
        for d.each |key, value| {
            if !first {
                wr.write_str(~", ");
            }
            first = false;
            wr.write_str(escape_str(key));
            wr.write_str(~": ");
            to_writer(wr, value);
        };
        wr.write_str(~" }");
      }
      Null => wr.write_str(~"null")
    }
}

fn escape_str(s: ~str) -> ~str {
    let mut escaped = ~"\"";
    do str::chars_iter(s) |c| {
        match c {
          '"' => escaped += ~"\\\"",
          '\\' => escaped += ~"\\\\",
          '\x08' => escaped += ~"\\b",
          '\x0c' => escaped += ~"\\f",
          '\n' => escaped += ~"\\n",
          '\r' => escaped += ~"\\r",
          '\t' => escaped += ~"\\t",
          _ => escaped += str::from_char(c)
        }
    };

    escaped += ~"\"";

    escaped
}

/// Serializes a json value into a string
fn to_str(j: Json) -> ~str {
    io::with_str_writer(|wr| to_writer(wr, j))
}

type Parser_ = {
    rdr: io::Reader,
    mut ch: char,
    mut line: uint,
    mut col: uint,
};

enum Parser {
    Parser_(Parser_)
}

impl Parser {
    fn eof() -> bool { self.ch == -1 as char }

    fn bump() {
        self.ch = self.rdr.read_char();

        if self.ch == '\n' {
            self.line += 1u;
            self.col = 1u;
        } else {
            self.col += 1u;
        }
    }

    fn next_char() -> char {
        self.bump();
        self.ch
    }

    fn error<T>(+msg: ~str) -> result<T, Error> {
        err({ line: self.line, col: self.col, msg: @msg })
    }

    fn parse() -> result<Json, Error> {
        match self.parse_value() {
          ok(value) => {
            // Skip trailing whitespaces.
            self.parse_whitespace();
            // Make sure there is no trailing characters.
            if self.eof() {
                ok(value)
            } else {
                self.error(~"trailing characters")
            }
          }
          e => e
        }
    }

    fn parse_value() -> result<Json, Error> {
        self.parse_whitespace();

        if self.eof() { return self.error(~"EOF while parsing value"); }

        match self.ch {
          'n' => self.parse_ident(~"ull", Null),
          't' => self.parse_ident(~"rue", Boolean(true)),
          'f' => self.parse_ident(~"alse", Boolean(false)),
          '0' to '9' | '-' => self.parse_number(),
          '"' => match self.parse_str() {
            ok(s) => ok(String(s)),
            err(e) => err(e)
          },
          '[' => self.parse_list(),
          '{' => self.parse_object(),
          _ => self.error(~"invalid syntax")
        }
    }

    fn parse_whitespace() {
        while char::is_whitespace(self.ch) { self.bump(); }
    }

    fn parse_ident(ident: ~str, value: Json) -> result<Json, Error> {
        if str::all(ident, |c| c == self.next_char()) {
            self.bump();
            ok(value)
        } else {
            self.error(~"invalid syntax")
        }
    }

    fn parse_number() -> result<Json, Error> {
        let mut neg = 1f;

        if self.ch == '-' {
            self.bump();
            neg = -1f;
        }

        let mut res = match self.parse_integer() {
          ok(res) => res,
          err(e) => return err(e)
        };

        if self.ch == '.' {
            match self.parse_decimal(res) {
              ok(r) => res = r,
              err(e) => return err(e)
            }
        }

        if self.ch == 'e' || self.ch == 'E' {
            match self.parse_exponent(res) {
              ok(r) => res = r,
              err(e) => return err(e)
            }
        }

        ok(Num(neg * res))
    }

    fn parse_integer() -> result<float, Error> {
        let mut res = 0f;

        match self.ch {
          '0' => {
            self.bump();

            // There can be only one leading '0'.
            match self.ch {
              '0' to '9' => return self.error(~"invalid number"),
              _ => ()
            }
          }
          '1' to '9' => {
            while !self.eof() {
                match self.ch {
                  '0' to '9' => {
                    res *= 10f;
                    res += ((self.ch as int) - ('0' as int)) as float;

                    self.bump();
                  }
                  _ => break
                }
            }
          }
          _ => return self.error(~"invalid number")
        }

        ok(res)
    }

    fn parse_decimal(res: float) -> result<float, Error> {
        self.bump();

        // Make sure a digit follows the decimal place.
        match self.ch {
          '0' to '9' => (),
          _ => return self.error(~"invalid number")
        }

        let mut res = res;
        let mut dec = 1f;
        while !self.eof() {
            match self.ch {
              '0' to '9' => {
                dec /= 10f;
                res += (((self.ch as int) - ('0' as int)) as float) * dec;

                self.bump();
              }
              _ => break
            }
        }

        ok(res)
    }

    fn parse_exponent(res: float) -> result<float, Error> {
        self.bump();

        let mut res = res;
        let mut exp = 0u;
        let mut neg_exp = false;

        match self.ch {
          '+' => self.bump(),
          '-' => { self.bump(); neg_exp = true; }
          _ => ()
        }

        // Make sure a digit follows the exponent place.
        match self.ch {
          '0' to '9' => (),
          _ => return self.error(~"invalid number")
        }

        while !self.eof() {
            match self.ch {
              '0' to '9' => {
                exp *= 10u;
                exp += (self.ch as uint) - ('0' as uint);

                self.bump();
              }
              _ => break
            }
        }

        let exp = float::pow_with_uint(10u, exp);
        if neg_exp {
            res /= exp;
        } else {
            res *= exp;
        }

        ok(res)
    }

    fn parse_str() -> result<@~str, Error> {
        let mut escape = false;
        let mut res = ~"";

        while !self.eof() {
            self.bump();

            if (escape) {
                match self.ch {
                  '"' => str::push_char(res, '"'),
                  '\\' => str::push_char(res, '\\'),
                  '/' => str::push_char(res, '/'),
                  'b' => str::push_char(res, '\x08'),
                  'f' => str::push_char(res, '\x0c'),
                  'n' => str::push_char(res, '\n'),
                  'r' => str::push_char(res, '\r'),
                  't' => str::push_char(res, '\t'),
                  'u' => {
                      // Parse \u1234.
                      let mut i = 0u;
                      let mut n = 0u;
                      while i < 4u {
                          match self.next_char() {
                            '0' to '9' => {
                              n = n * 10u +
                                  (self.ch as uint) - ('0' as uint);
                            }
                            _ => return self.error(~"invalid \\u escape")
                          }
                          i += 1u;
                      }

                      // Error out if we didn't parse 4 digits.
                      if i != 4u {
                          return self.error(~"invalid \\u escape");
                      }

                      str::push_char(res, n as char);
                  }
                  _ => return self.error(~"invalid escape")
                }
                escape = false;
            } else if self.ch == '\\' {
                escape = true;
            } else {
                if self.ch == '"' {
                    self.bump();
                    return ok(@res);
                }
                str::push_char(res, self.ch);
            }
        }

        self.error(~"EOF while parsing string")
    }

    fn parse_list() -> result<Json, Error> {
        self.bump();
        self.parse_whitespace();

        let mut values = ~[];

        if self.ch == ']' {
            self.bump();
            return ok(List(@values));
        }

        loop {
            match self.parse_value() {
              ok(v) => vec::push(values, v),
              e => return e
            }

            self.parse_whitespace();
            if self.eof() {
                return self.error(~"EOF while parsing list");
            }

            match self.ch {
              ',' => self.bump(),
              ']' => { self.bump(); return ok(List(@values)); }
              _ => return self.error(~"expected `,` or `]`")
            }
        };
    }

    fn parse_object() -> result<Json, Error> {
        self.bump();
        self.parse_whitespace();

        let values = map::str_hash();

        if self.ch == '}' {
          self.bump();
          return ok(Dict(values));
        }

        while !self.eof() {
            self.parse_whitespace();

            if self.ch != '"' {
                return self.error(~"key must be a string");
            }

            let key = match self.parse_str() {
              ok(key) => key,
              err(e) => return err(e)
            };

            self.parse_whitespace();

            if self.ch != ':' {
                if self.eof() { break; }
                return self.error(~"expected `:`");
            }
            self.bump();

            match self.parse_value() {
              ok(value) => { values.insert(copy *key, value); }
              e => return e
            }
            self.parse_whitespace();

            match self.ch {
              ',' => self.bump(),
              '}' => { self.bump(); return ok(Dict(values)); }
              _ => {
                  if self.eof() { break; }
                  return self.error(~"expected `,` or `}`");
              }
            }
        }

        return self.error(~"EOF while parsing object");
    }
}

/// Deserializes a json value from an io::reader
fn from_reader(rdr: io::Reader) -> result<Json, Error> {
    let parser = Parser_({
        rdr: rdr,
        mut ch: rdr.read_char(),
        mut line: 1u,
        mut col: 1u,
    });

    parser.parse()
}

/// Deserializes a json value from a string
fn from_str(s: ~str) -> result<Json, Error> {
    io::with_str_reader(s, from_reader)
}

/// Test if two json values are equal
fn eq(value0: Json, value1: Json) -> bool {
    match (value0, value1) {
      (Num(f0), Num(f1)) => f0 == f1,
      (String(s0), String(s1)) => s0 == s1,
      (Boolean(b0), Boolean(b1)) => b0 == b1,
      (List(l0), List(l1)) => vec::all2(*l0, *l1, eq),
      (Dict(d0), Dict(d1)) => {
          if d0.size() == d1.size() {
              let mut equal = true;
              for d0.each |k, v0| {
                  match d1.find(k) {
                    some(v1) => if !eq(v0, v1) { equal = false },
                    none => equal = false
                  }
              };
              equal
          } else {
              false
          }
      }
      (Null, Null) => true,
      _ => false
    }
}

trait ToJson { fn to_json() -> Json; }

impl Json: ToJson {
    fn to_json() -> Json { self }
}

impl @Json: ToJson {
    fn to_json() -> Json { *self }
}

impl int: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl i8: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl i16: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl i32: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl i64: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl uint: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl u8: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl u16: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl u32: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl u64: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl float: ToJson {
    fn to_json() -> Json { Num(self) }
}

impl f32: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl f64: ToJson {
    fn to_json() -> Json { Num(self as float) }
}

impl (): ToJson {
    fn to_json() -> Json { Null }
}

impl bool: ToJson {
    fn to_json() -> Json { Boolean(self) }
}

impl ~str: ToJson {
    fn to_json() -> Json { String(@copy self) }
}

impl @~str: ToJson {
    fn to_json() -> Json { String(self) }
}

impl <A: ToJson, B: ToJson> (A, B): ToJson {
    fn to_json() -> Json {
        match self {
          (a, b) => {
            List(@~[a.to_json(), b.to_json()])
          }
        }
    }
}

impl <A: ToJson, B: ToJson, C: ToJson> (A, B, C): ToJson {

    fn to_json() -> Json {
        match self {
          (a, b, c) => {
            List(@~[a.to_json(), b.to_json(), c.to_json()])
          }
        }
    }
}

impl <A: ToJson> ~[A]: ToJson {
    fn to_json() -> Json { List(@self.map(|elt| elt.to_json())) }
}

impl <A: ToJson copy> hashmap<~str, A>: ToJson {
    fn to_json() -> Json {
        let d = map::str_hash();
        for self.each() |key, value| {
            d.insert(copy key, value.to_json());
        }
        Dict(d)
    }
}

impl <A: ToJson> option<A>: ToJson {
    fn to_json() -> Json {
        match self {
          none => Null,
          some(value) => value.to_json()
        }
    }
}

impl Json: to_str::ToStr {
    fn to_str() -> ~str { to_str(self) }
}

impl Error: to_str::ToStr {
    fn to_str() -> ~str {
        fmt!{"%u:%u: %s", self.line, self.col, *self.msg}
    }
}

#[cfg(test)]
mod tests {
    fn mk_dict(items: ~[(~str, Json)]) -> Json {
        let d = map::str_hash();

        do vec::iter(items) |item| {
            let (key, value) = copy item;
            d.insert(key, value);
        };

        Dict(d)
    }

    #[test]
    fn test_write_null() {
        assert to_str(Null) == ~"null";
    }

    #[test]
    fn test_write_num() {
        assert to_str(Num(3f)) == ~"3";
        assert to_str(Num(3.1f)) == ~"3.1";
        assert to_str(Num(-1.5f)) == ~"-1.5";
        assert to_str(Num(0.5f)) == ~"0.5";
    }

    #[test]
    fn test_write_str() {
        assert to_str(String(@~"")) == ~"\"\"";
        assert to_str(String(@~"foo")) == ~"\"foo\"";
    }

    #[test]
    fn test_write_bool() {
        assert to_str(Boolean(true)) == ~"true";
        assert to_str(Boolean(false)) == ~"false";
    }

    #[test]
    fn test_write_list() {
        assert to_str(List(@~[])) == ~"[]";
        assert to_str(List(@~[Boolean(true)])) == ~"[true]";
        assert to_str(List(@~[
            Boolean(false),
            Null,
            List(@~[String(@~"foo\nbar"), Num(3.5f)])
        ])) == ~"[false, null, [\"foo\\nbar\", 3.5]]";
    }

    #[test]
    fn test_write_dict() {
        assert to_str(mk_dict(~[])) == ~"{}";
        assert to_str(mk_dict(~[(~"a", Boolean(true))]))
            == ~"{ \"a\": true }";
        assert to_str(mk_dict(~[
            (~"a", Boolean(true)),
            (~"b", List(@~[
                mk_dict(~[(~"c", String(@~"\x0c\r"))]),
                mk_dict(~[(~"d", String(@~""))])
            ]))
        ])) ==
            ~"{ " +
                ~"\"a\": true, " +
                ~"\"b\": [" +
                    ~"{ \"c\": \"\\f\\r\" }, " +
                    ~"{ \"d\": \"\" }" +
                ~"]" +
            ~" }";
    }

    #[test]
    fn test_trailing_characters() {
        assert from_str(~"nulla") ==
            err({line: 1u, col: 5u, msg: @~"trailing characters"});
        assert from_str(~"truea") ==
            err({line: 1u, col: 5u, msg: @~"trailing characters"});
        assert from_str(~"falsea") ==
            err({line: 1u, col: 6u, msg: @~"trailing characters"});
        assert from_str(~"1a") ==
            err({line: 1u, col: 2u, msg: @~"trailing characters"});
        assert from_str(~"[]a") ==
            err({line: 1u, col: 3u, msg: @~"trailing characters"});
        assert from_str(~"{}a") ==
            err({line: 1u, col: 3u, msg: @~"trailing characters"});
    }

    #[test]
    fn test_read_identifiers() {
        assert from_str(~"n") ==
            err({line: 1u, col: 2u, msg: @~"invalid syntax"});
        assert from_str(~"nul") ==
            err({line: 1u, col: 4u, msg: @~"invalid syntax"});

        assert from_str(~"t") ==
            err({line: 1u, col: 2u, msg: @~"invalid syntax"});
        assert from_str(~"truz") ==
            err({line: 1u, col: 4u, msg: @~"invalid syntax"});

        assert from_str(~"f") ==
            err({line: 1u, col: 2u, msg: @~"invalid syntax"});
        assert from_str(~"faz") ==
            err({line: 1u, col: 3u, msg: @~"invalid syntax"});

        assert from_str(~"null") == ok(Null);
        assert from_str(~"true") == ok(Boolean(true));
        assert from_str(~"false") == ok(Boolean(false));
        assert from_str(~" null ") == ok(Null);
        assert from_str(~" true ") == ok(Boolean(true));
        assert from_str(~" false ") == ok(Boolean(false));
    }

    #[test]
    fn test_read_num() {
        assert from_str(~"+") ==
            err({line: 1u, col: 1u, msg: @~"invalid syntax"});
        assert from_str(~".") ==
            err({line: 1u, col: 1u, msg: @~"invalid syntax"});

        assert from_str(~"-") ==
            err({line: 1u, col: 2u, msg: @~"invalid number"});
        assert from_str(~"00") ==
            err({line: 1u, col: 2u, msg: @~"invalid number"});
        assert from_str(~"1.") ==
            err({line: 1u, col: 3u, msg: @~"invalid number"});
        assert from_str(~"1e") ==
            err({line: 1u, col: 3u, msg: @~"invalid number"});
        assert from_str(~"1e+") ==
            err({line: 1u, col: 4u, msg: @~"invalid number"});

        assert from_str(~"3") == ok(Num(3f));
        assert from_str(~"3.1") == ok(Num(3.1f));
        assert from_str(~"-1.2") == ok(Num(-1.2f));
        assert from_str(~"0.4") == ok(Num(0.4f));
        assert from_str(~"0.4e5") == ok(Num(0.4e5f));
        assert from_str(~"0.4e+15") == ok(Num(0.4e15f));
        assert from_str(~"0.4e-01") == ok(Num(0.4e-01f));
        assert from_str(~" 3 ") == ok(Num(3f));
    }

    #[test]
    fn test_read_str() {
        assert from_str(~"\"") ==
            err({line: 1u, col: 2u, msg: @~"EOF while parsing string"});
        assert from_str(~"\"lol") ==
            err({line: 1u, col: 5u, msg: @~"EOF while parsing string"});

        assert from_str(~"\"\"") == ok(String(@~""));
        assert from_str(~"\"foo\"") == ok(String(@~"foo"));
        assert from_str(~"\"\\\"\"") == ok(String(@~"\""));
        assert from_str(~"\"\\b\"") == ok(String(@~"\x08"));
        assert from_str(~"\"\\n\"") == ok(String(@~"\n"));
        assert from_str(~"\"\\r\"") == ok(String(@~"\r"));
        assert from_str(~"\"\\t\"") == ok(String(@~"\t"));
        assert from_str(~" \"foo\" ") == ok(String(@~"foo"));
    }

    #[test]
    fn test_read_list() {
        assert from_str(~"[") ==
            err({line: 1u, col: 2u, msg: @~"EOF while parsing value"});
        assert from_str(~"[1") ==
            err({line: 1u, col: 3u, msg: @~"EOF while parsing list"});
        assert from_str(~"[1,") ==
            err({line: 1u, col: 4u, msg: @~"EOF while parsing value"});
        assert from_str(~"[1,]") ==
            err({line: 1u, col: 4u, msg: @~"invalid syntax"});
        assert from_str(~"[6 7]") ==
            err({line: 1u, col: 4u, msg: @~"expected `,` or `]`"});

        assert from_str(~"[]") == ok(List(@~[]));
        assert from_str(~"[ ]") == ok(List(@~[]));
        assert from_str(~"[true]") == ok(List(@~[Boolean(true)]));
        assert from_str(~"[ false ]") == ok(List(@~[Boolean(false)]));
        assert from_str(~"[null]") == ok(List(@~[Null]));
        assert from_str(~"[3, 1]") == ok(List(@~[Num(3f), Num(1f)]));
        assert from_str(~"\n[3, 2]\n") == ok(List(@~[Num(3f), Num(2f)]));
        assert from_str(~"[2, [4, 1]]") ==
               ok(List(@~[Num(2f), List(@~[Num(4f), Num(1f)])]));
    }

    #[test]
    fn test_read_dict() {
        assert from_str(~"{") ==
            err({line: 1u, col: 2u, msg: @~"EOF while parsing object"});
        assert from_str(~"{ ") ==
            err({line: 1u, col: 3u, msg: @~"EOF while parsing object"});
        assert from_str(~"{1") ==
            err({line: 1u, col: 2u, msg: @~"key must be a string"});
        assert from_str(~"{ \"a\"") ==
            err({line: 1u, col: 6u, msg: @~"EOF while parsing object"});
        assert from_str(~"{\"a\"") ==
            err({line: 1u, col: 5u, msg: @~"EOF while parsing object"});
        assert from_str(~"{\"a\" ") ==
            err({line: 1u, col: 6u, msg: @~"EOF while parsing object"});

        assert from_str(~"{\"a\" 1") ==
            err({line: 1u, col: 6u, msg: @~"expected `:`"});
        assert from_str(~"{\"a\":") ==
            err({line: 1u, col: 6u, msg: @~"EOF while parsing value"});
        assert from_str(~"{\"a\":1") ==
            err({line: 1u, col: 7u, msg: @~"EOF while parsing object"});
        assert from_str(~"{\"a\":1 1") ==
            err({line: 1u, col: 8u, msg: @~"expected `,` or `}`"});
        assert from_str(~"{\"a\":1,") ==
            err({line: 1u, col: 8u, msg: @~"EOF while parsing object"});

        assert eq(result::get(from_str(~"{}")), mk_dict(~[]));
        assert eq(result::get(from_str(~"{\"a\": 3}")),
                  mk_dict(~[(~"a", Num(3.0f))]));

        assert eq(result::get(from_str(~"{ \"a\": null, \"b\" : true }")),
                  mk_dict(~[
                      (~"a", Null),
                      (~"b", Boolean(true))]));
        assert eq(result::get(from_str(~"\n{ \"a\": null, \"b\" : true }\n")),
                  mk_dict(~[
                      (~"a", Null),
                      (~"b", Boolean(true))]));
        assert eq(result::get(from_str(~"{\"a\" : 1.0 ,\"b\": [ true ]}")),
                  mk_dict(~[
                      (~"a", Num(1.0)),
                      (~"b", List(@~[Boolean(true)]))
                  ]));
        assert eq(result::get(from_str(
                      ~"{" +
                          ~"\"a\": 1.0, " +
                          ~"\"b\": [" +
                              ~"true," +
                              ~"\"foo\\nbar\", " +
                              ~"{ \"c\": {\"d\": null} } " +
                          ~"]" +
                      ~"}")),
                  mk_dict(~[
                      (~"a", Num(1.0f)),
                      (~"b", List(@~[
                          Boolean(true),
                          String(@~"foo\nbar"),
                          mk_dict(~[
                              (~"c", mk_dict(~[(~"d", Null)]))
                          ])
                      ]))
                  ]));
    }

    #[test]
    fn test_multiline_errors() {
        assert from_str(~"{\n  \"foo\":\n \"bar\"") ==
            err({line: 3u, col: 8u, msg: @~"EOF while parsing object"});
    }
}
