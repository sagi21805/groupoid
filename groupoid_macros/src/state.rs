use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub(crate) struct State<'ast> {
    item_struct: &'ast ItemStruct,
}

impl<'ast> State<'ast> {
    pub(crate) fn new(item_struct: &'ast ItemStruct) -> State<'ast> {
        State { item_struct }
    }

    pub(crate) fn generate_state_impl(&self) -> TokenStream {
        let item_struct = self.item_struct;
        let struct_ident = &item_struct.ident;
        let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();

        quote! {
            #item_struct

            impl #impl_generics ::groupoid::State for #struct_ident #ty_generics #where_clause {}
        }
    }
}
