use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use syn::parse_macro_input;

lazy_static! {
    static ref MSGTYPE_SIZE_MAP: HashMap<&'static str, usize> = {
        let mut m = HashMap::new();
        m.insert("int8", 1);
        m.insert("int16", 2);
        m.insert("int32", 4);
        m.insert("int64", 8);
        m.insert("uint8", 1);
        m.insert("uint16", 2);
        m.insert("uint32", 4);
        m.insert("uint64", 8);
        m.insert("float32", 4);
        m.insert("float64", 8);
        m.insert("bool", 1);
        m.insert("char", 1);
        m
    };
}

fn hash_32_fnv1a(data: &str) -> u32 {
    let mut hash_val: u32 = 0x811c9dc5;
    let prime: u32 = 0x1000193;

    for byte in data.bytes() {
        hash_val ^= byte as u32;
        hash_val = hash_val.wrapping_mul(prime);
    }

    hash_val
}

pub fn get_message_hash(
    members: &[(syn::Ident, usize, proc_macro2::TokenStream, String, usize)],
) -> u32 {
    let all_fields_str = get_message_fields_str_for_message_hash(members);
    hash_32_fnv1a(&all_fields_str)
}

fn get_message_fields_str_for_message_hash(
    members: &[(syn::Ident, usize, proc_macro2::TokenStream, String, usize)],
) -> String {
    let mut all_fields_str = String::new();

    for (name, _, rust_type, c_type, _) in members {
        // c_type'ı kullanarak tip bilgisini alıyoruz
        let type_str = c_type.split('[').next().unwrap_or(c_type);
        // name'i string'e çeviriyoruz
        let name_str = name.to_string();

        all_fields_str.push_str(&format!("{} {}\n", type_str, name_str));
    }

    all_fields_str
}

pub fn px4_message(args: TokenStream, input: TokenStream) -> TokenStream {
    let arg = parse_macro_input!(args as syn::LitStr).value();

    // Open .msg file

    let path = if let Some(root) = std::env::var_os("CARGO_MANIFEST_DIR") {
        Path::new(&root).join(&arg)
    } else {
        arg.into()
    };
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("Unable to open {:?}: {}", path, e);
    });
    let file = BufReader::new(file);

    // Verify that the struct looks like `[pub] struct name;`

    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = input.ident;
    let is_unit_struct = match input.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Unit => true,
            _ => false,
        },
        _ => false,
    };
    if !is_unit_struct || input.generics.lt_token.is_some() {
        panic!("Expected `struct {};`", name);
    }

    // Read the .msg file line by line, collecting all the struct members.

    let mut members = Vec::new();
    let mut constants = Vec::new();

    for (line_num, line) in file.lines().enumerate() {
        // Parse the lines, throwing away comments and empty lines, splitting them in type and name.

        let mut line = line.unwrap_or_else(|e| {
            panic!("Unable to read from {:?}: {}", path, e);
        });
        if let Some(comment_start) = line.find('#') {
            line.truncate(comment_start);
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse constant types
        if let Some(eq_pos) = line.find("=") {
            // Parse constant definitions
            let (type_and_name, value) = line.split_at(eq_pos);
            let value = value[1..].trim();
            let mut words = type_and_name.split_whitespace();
            let type_ = words.next().unwrap();
            let name = words.next().unwrap_or_else(|| {
                panic!(
                    "Missing constant name on line {} in {:?}",
                    line_num + 1,
                    path
                );
            });
            if words.next().is_some() {
                panic!(
                    "Garbage after constant name on line {} in {:?}",
                    line_num + 1,
                    path
                );
            }

            let constant_name = syn::Ident::new(name, proc_macro2::Span::call_site());
            let constant_tokens = match type_ {
                "uint8" | "byte" => {
                    let val = value.parse::<u8>().unwrap();
                    quote! {
                        pub const #constant_name: u8 = #val;
                    }
                }
                "uint16" => {
                    let val = value.parse::<u16>().unwrap();
                    quote! {
                        pub const #constant_name: u16 = #val;
                    }
                }
                "uint32" => {
                    let val = value.parse::<u32>().unwrap();
                    quote! {
                        pub const #constant_name: u32 = #val;
                    }
                }
                "uint64" => {
                    let val = value.parse::<u64>().unwrap();
                    quote! {
                        pub const #constant_name: u64 = #val;
                    }
                }
                "int8" => {
                    let val = value.parse::<i8>().unwrap();
                    quote! {
                        pub const #constant_name: i8 = #val;
                    }
                }
                "int16" => {
                    let val = value.parse::<i16>().unwrap();
                    quote! {
                        pub const #constant_name: i16 = #val;
                    }
                }
                "int32" => {
                    let val = value.parse::<i32>().unwrap();
                    quote! {
                        pub const #constant_name: i32 = #val;
                    }
                }
                "int64" => {
                    let val = value.parse::<i64>().unwrap();
                    quote! {
                        pub const #constant_name: i64 = #val;
                    }
                }
                "float32" => {
                    let val = value.parse::<f32>().unwrap();
                    quote! {
                        pub const #constant_name: f32 = #val;
                    }
                }
                "float64" => {
                    let val = value.parse::<f64>().unwrap();
                    quote! {
                        pub const #constant_name: f64 = #val;
                    }
                }
                "bool" => {
                    let val = value.parse::<bool>().unwrap();
                    quote! {
                        pub const #constant_name: bool = #val;
                    }
                }
                _ => panic!(
                    "Unknown type `{}` for constant on line {} in {:?}",
                    type_,
                    line_num + 1,
                    path
                ),
            };
            constants.push(constant_tokens);
        } else {
            let mut words = line.split_whitespace();
            let mut type_ = words
                .next()
                .unwrap_or_else(|| panic!("type alinmasi lazim!!!"));
            let name = words.next().unwrap_or_else(|| {
                panic!("Missing name on line {} in {:?}", line_num + 1, path);
            });

            // Parse array types.

            let array_len = type_.find('[').map(|open_brace| {
                if !type_.ends_with(']') {
                    panic!("Missing `]` on line {} in {:?}", line_num + 1, path);
                }
                let braced_part = &type_[open_brace + 1..type_.len() - 1];
                type_ = &type_[..open_brace];
                braced_part.parse::<usize>().unwrap_or_else(|_| {
                    panic!(
                        "Invalid array length on line {} in {:?}",
                        line_num + 1,
                        path
                    );
                })
            });

            // Look up the type's width, Rust type, and C type.

            let (width, mut rust_type, c_type) = match type_ {
                "uint64" => (8, quote! { u64  }, "uint64_t"),
                "uint32" => (4, quote! { u32  }, "uint32_t"),
                "uint16" => (2, quote! { u16  }, "uint16_t"),
                "uint8" | "byte" => (1, quote! { u8   }, "uint8_t"),
                "int64" => (8, quote! { i64  }, "int64_t"),
                "int32" => (4, quote! { i32  }, "int32_t"),
                "int16" => (2, quote! { i16  }, "int16_t"),
                "int8" => (1, quote! { i8   }, "int8_t"),
                "float64" => (8, quote! { f64  }, "double"),
                "float32" => (4, quote! { f32  }, "float"),
                "char" => (1, quote! { u8   }, "char"),
                "bool" => (1, quote! { bool }, "bool"),
                _ => {
                    panic!(
                        "Unknown type `{}` on line {} in {:?}",
                        type_,
                        line_num + 1,
                        path
                    );
                }
            };
            let mut c = c_type.to_string();
            if let Some(n) = array_len {
                rust_type = quote! { [#rust_type; #n] };
                c = format!("{}[{}]", c, n);
            }

            // Add it to the list.

            let name = syn::Ident::new(name, Span::call_site());
            let size = array_len.unwrap_or(1) * width;
            members.push((name, width, rust_type, c, size));
        }
    }

    // Sort the members by alignment, biggest first.

    members.sort_by(|a, b| b.1.cmp(&a.1));

    // Compute the total size and generate the message fields description.

    let mut fields = String::new();
    let mut size = 0;
    for (name, _, _, c_type, field_size) in &members {
        write!(&mut fields, "{} {};", c_type, name)
            .unwrap_or_else(|error| panic!("nasil olmazzzzz {error}"));
        size += field_size;
    }
    let size_no_padding = size;
    // Add padding if the size is not a multiple of 8 yet.
    // (Note that since we sort the fields by their alignment, and each bigger
    // alignment is a multiple of each smaller alignment; padding can not
    // occur between fields, only at the end.)
    if size % 8 != 0 {
        let padding = 8 - size % 8;
        write!(fields, "uint8_t[{}] _padding0;", padding).unwrap();
        size += padding;
    }
    fields.push('\0');

    if size > 0xFFFF {
        panic!("Message size too big");
    }

    // Generate the Rust code.

    let vis = input.vis;
    let attrs = input.attrs;
    let mems = members
        .iter()
        .map(|(name, _, ty, _, _)| quote! { #name: #ty });
    let path = path.to_str().unwrap();
    let name_str = format!("{}\0", name);
    let size = size as u16;
    let size_no_padding = size_no_padding as u16;
    let message_hash = get_message_hash(&members);
    let queue = 1u8;
    let id = 1u16;

    let expanded = quote! {
        #[repr(C)]
        #[repr(align(8))]
        #[derive(Clone, Debug)]
        #(#attrs)*
        #vis struct #name {
            #(#mems),*
        }
        impl #name {
            #(#constants)*
        }
         unsafe impl Message for #name {
             fn metadata() -> &'static c::Metadata {
                 let _ = include_bytes!(#path); // This causes the file to be recompiled if the .msg-file is changed.
                 static M: c::Metadata = c::Metadata::_unsafe_new(
                     #name_str as *const str as *const u8,
                     #size,
                     #size_no_padding,
                     #message_hash,
                     #id,
                     #queue
                 );
                 &M
             }
         }
    };

    expanded.into()
}
