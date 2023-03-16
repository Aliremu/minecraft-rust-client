use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use quote::{TokenStreamExt, ToTokens};
use syn::{Type, ReturnType};
use syn::{self, ext::IdentExt, spanned::Spanned, Field, Lit, Meta, MetaNameValue, Visibility};

fn get_jvalue(name: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("JValue::Bool({} as u8)", name),
        "char" => format!("JValue::Char({})", name),
        "i32" => format!("JValue::Int({})", name),
        "f32" => format!("JValue::Float({})", name),
        "f64" => format!("JValue::Double({})", name),
        "& str" => format!("JValue::from(&env.new_string({})?)", name),
        _ => format!("JValue::Object(&({}.instance.as_ref().unwrap()))", name)
    }
}

pub fn generate_field(field_name: &Ident, field_ty: &Type, attr_name: &str, attr_ty: &str, is_static: bool) -> TokenStream2 {
    let get_fn_name = &format_ident!("get_{}{}", field_name, if is_static { "_static" } else { "" });
    let set_fn_name = &format_ident!("set_{}{}", field_name,  if is_static { "_static" } else { "" });

    println!("{} {} {}", field_name, attr_name, set_fn_name);

    let mut import_get = quote! {};
    let mut import_set = quote! {};
    let mut import_object = quote!{};

    let j_type = Ident::new(&attr_ty[0..1].to_lowercase(), Span::call_site());

    if j_type == "l" {
        import_object = quote! {
            let mut object = #field_ty::new(self.app.clone())?;
            object.set_instance(out);

            let out = object;
        }
    }

    let j_value: proc_macro2::TokenStream = get_jvalue("val", quote!{#field_ty}.to_string().as_str()).parse().unwrap();

    if is_static {
        import_get = quote! {
            let out = env.get_static_field(
                &(self.class),
                #attr_name,
                #attr_ty
            )?.#j_type()?;
        };

        import_set = quote! {
            let id = env.get_static_field_id(&(self.class), #attr_name, #attr_ty)?;
            let out = env.set_static_field(
                &(self.class), 
                id,
                #j_value
            )?;
        };
    } else {
        import_get = quote! {
            let out = env.get_field(
                &(self.instance.as_ref().unwrap()),
                #attr_name,
                #attr_ty
            )?.#j_type()?;
        };

        import_set = quote! {
            let out = env.set_field(
                &(self.instance.as_ref().unwrap()),
                #attr_name,
                #attr_ty,
                #j_value
            )?;
        }
    }

    quote! {
        pub unsafe fn #get_fn_name(&self) -> Result<#field_ty, jni::errors::Error> {
            let mut env = self.app.get_env()?;

            #import_get
            #import_object

            Ok(out)
        }

        pub unsafe fn #set_fn_name(&self, val: #field_ty) -> Result<(), jni::errors::Error> {
            let mut env = self.app.get_env()?;
            
            #import_set

            Ok(())
        }
    }
}



pub fn generate_method(method_name: &Ident, method_ty: &ReturnType, method_args: Vec<(Ident, Type)>, attr_name: &str, attr_sig: &str, is_static: bool) -> TokenStream2 {
    if let ReturnType::Type(_, output) = &method_ty {
        let fn_name = &format_ident!("{}{}", method_name, if is_static { "_static" } else { "" });

        let mut args = method_args.iter().map(|x| {
            let name = &x.0;
            let ty = &x.1;
            quote! { , #name: #ty }
        }).collect::<TokenStream2>();

        let mut j_args = method_args.iter().map(|x| {
            let name = &x.0;
            let ty = &x.1;
            get_jvalue(quote! {#name}.to_string().as_str(), quote! {#ty}.to_string().as_str())
        }).collect::<Vec<String>>();

        let j_args: TokenStream2 = j_args.join(",").parse().unwrap();

        let mut import_object = quote! {};
        let mut import_method = quote! {};

        let j_type: TokenStream2 = match quote!{ #output }.to_string().as_str() {
            "bool" => ".z()?",
            "char" => ".c()?",
            "i32" => ".i()?",
            "f64" => ".f()?",
            "f64" => ".d()?",
            "()" => {
                import_object = quote! { let out = (); };
                ""
            },
            _ => {
                import_object = quote! {
                    let mut object = #output::new(self.app)?;
                    object.set_instance(out);
    
                    let out = object;
                };

                ".l()?"
            }
        }.parse().unwrap();

        if is_static {
            import_method = quote! {
                let out = env
                .call_static_method(
                    &(self.class),
                    #attr_name,
                    #attr_sig,
                    &[#j_args]
                )?#j_type;
            }
        } else {
            import_method = quote! {
                let out = env
                .call_method(
                    &(self.instance.as_ref().unwrap()),
                    #attr_name,
                    #attr_sig,
                    &[#j_args]
                )?#j_type;
            }
        }
        
        let gen = quote! {
            pub unsafe fn #fn_name(&self #args) -> Result<#output, jni::errors::Error> {
                let mut env = self.app.get_env()?;

                #import_method

                #import_object

                Ok(out)
            }
        };

        println!("{:?}", gen.to_string());
        return gen;
    }
    
    quote! {}
}