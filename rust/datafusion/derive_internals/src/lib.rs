extern crate proc_macro;
use self::proc_macro::TokenStream;

use proc_macro_hack::proc_macro_hack;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, parse_quote, Block, FnArg, Ident, ItemFn, Pat, PatType,
    ReturnType, Token, Type, TypePath,
};

#[derive(Debug, Clone)]
struct Arg {
    pub ty: Ident,
    pub name: Ident,
}

struct UDF {
    pub args: Vec<Arg>,
    pub inner: ItemFn,
    pub name: String,
    pub output: String,
    pub meta: Option<UDFMeta>,
}

struct UDFMeta {
    pub arg_arr_types: Vec<syn::Type>,
    pub arg_arr_str_types: Vec<String>,
    pub var_names: Vec<Ident>,
    pub output_item_type: syn::Type,
    pub output_arr_type: syn::Type,
    pub arr_len: proc_macro2::TokenStream,
    pub get_item_value: Vec<proc_macro2::TokenStream>,
}

impl Parse for UDF {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner: ItemFn = input.parse()?;

        let args: Vec<Arg> = inner
            .sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                FnArg::Typed(pt) => Some(pt),
                _ => None,
            })
            .cloned()
            .filter_map(|pt| match (*pt.ty, *pt.pat) {
                (Type::Path(ty), Pat::Ident(pat)) => Some((ty.path, pat)),
                _ => None,
            })
            .map(|(ty, pat)| (ty.get_ident().unwrap().clone(), pat.ident.to_string())) //pat.ident.to_string())
            .map(|(ty, name)| Arg {
                ty: format_ident!("{}", ty),
                name: format_ident!("{}", name),
            })
            .collect::<Vec<Arg>>();

        let name = inner.sig.ident.to_string();
        let output = inner.sig.output.clone();

        let output = match output {
            ReturnType::Type(_, pat) => match *pat {
                Type::Path(ty) => ty.path,
                _ => unreachable!("ReturnType should be Type::Path"),
            },
            _ => panic!("UDF should return value"),
        }
        .get_ident()
        .unwrap()
        .to_string();

        Ok(UDF {
            args,
            inner,
            name,
            output,
            meta: None,
        })
    }
}

fn ty_to_arr_type(ty: &str) -> Result<syn::Type> {
    match ty {
        "u8" => syn::parse_str::<syn::Type>("arrow::array::UInt8Array"),
        "u16" => syn::parse_str::<syn::Type>("arrow::array::UInt16Array"),
        "u32" => syn::parse_str::<syn::Type>("arrow::array::UInt32Array"),
        "u64" => syn::parse_str::<syn::Type>("arrow::array::UInt64Array"),
        "i8" => syn::parse_str::<syn::Type>("arrow::array::Int8Array"),
        "i16" => syn::parse_str::<syn::Type>("arrow::array::Int16Array"),
        "i32" => syn::parse_str::<syn::Type>("arrow::array::Int32Array"),
        "i64" => syn::parse_str::<syn::Type>("arrow::array::Int64Array"),
        "f32" => syn::parse_str::<syn::Type>("arrow::array::Float32Array"),
        "f64" => syn::parse_str::<syn::Type>("arrow::array::Float64Array"),
        "String" => syn::parse_str::<syn::Type>("arrow::array::BinaryArray"),
        _ => panic!("{} does not impl ArrayPrimitiveType", ty),
    }
}

fn make_udf(udf: &mut UDF) -> proc_macro2::TokenStream {
    let arr_type_iter = udf.args.iter().map(|arg| arg.ty.to_string());

    let arg_arr_types = arr_type_iter
        .clone()
        .map(|ty| {
            ty_to_arr_type(&ty).expect("Could not convert arg type into arrow array type")
        })
        .collect::<Vec<syn::Type>>();

    let arg_arr_str_types = arr_type_iter.collect::<Vec<String>>();

    let var_names = udf
        .args
        .iter()
        .cloned()
        .map(|arg| arg.name)
        .collect::<Vec<Ident>>();

    let value_getter = |var: &Ident, ty: &str| match ty {
        "String" => {
            quote! { #var.get_string(i) }
        }
        _ => {
            quote! { #var.value(i) }
        }
    };

    let fn_name = format_ident!("{}", udf.name);

    let arg_idx = 0..var_names.len();

    let first_param = var_names
        .get(0)
        .expect("UDF should contain at least one parameter")
        .clone();

    let output_item_type = syn::parse_str::<syn::Type>(&udf.output)
        .expect("Should be able to convert output to type");
    let output_arr_type = ty_to_arr_type(&udf.output).unwrap();

    let get_item_value = arg_arr_str_types
        .iter()
        .enumerate()
        .map(|(i, ty)| value_getter(&var_names[i], ty))
        .collect::<Vec<proc_macro2::TokenStream>>();

    let arr_len = quote! {
        #first_param.len()
    };

    let udf_inner = quote! {
        use arrow::array::Array;

        let ( #(#var_names),* ): ( #(&#arg_arr_types),* ) = ( #( data[#arg_idx].as_any().downcast_ref::<#arg_arr_types>().unwrap() ),* );

        assert!(
            #(#var_names.len()) == *,
            "arrays should be of the same length"
        );

        let len = #arr_len;
        let mut out: Vec<#output_item_type> = Vec::with_capacity(len);

        for i in 0..len {
            out.push(#fn_name( #(#get_item_value),* ));
        }

        let out_arr: #output_arr_type = out.into();
        Arc::new(out_arr)
    };

    udf.meta = Some(UDFMeta {
        arg_arr_types,
        arg_arr_str_types,
        var_names,
        output_item_type,
        output_arr_type,
        get_item_value,
        arr_len,
    });

    udf_inner
}

#[proc_macro_attribute]
pub fn derive_udf(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut udf = parse_macro_input!(item as UDF);
    let udf_inner = make_udf(&mut udf);

    let udf_fn_name = format_ident!("{}_udf", udf.name);
    let udf_wrapper = quote! {
        pub fn #udf_fn_name(data: &[arrow::array::ArrayRef]) -> ArrayRef {
            #udf_inner
        }
    };

    let func = udf.inner;

    TokenStream::from(quote! {
        #func
        #udf_wrapper
    })
}

#[proc_macro_hack]
pub fn compose_udf(item: TokenStream) -> TokenStream {
    let mut udf = parse_macro_input!(item as UDF);
    let udf_inner = make_udf(&mut udf);

    TokenStream::from(quote! {
        |data: &[arrow::array::ArrayRef]| -> ArrayRef {
            #udf_inner
        }
    })
}
