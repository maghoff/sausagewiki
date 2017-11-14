use std::fs::File;

use proc_macro::TokenStream;
use quote;
use serde_json;
use serde::de::IgnoredAny;

const SOURCES: &[&str] = &[
    "src/licenses/license-hound.json",
    "src/licenses/other.json",
];

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum LicenseId {
    Bsd3Clause,
    Mit,
    Mpl2,
    Ofl11,
}

impl quote::ToTokens for LicenseId {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        use self::LicenseId::*;
        tokens.append(match self {
            &Bsd3Clause => "Bsd3Clause",
            &Mit => "Mit",
            &Mpl2 => "Mpl2",
            &Ofl11 => "Ofl11",
        });
    }
}

#[derive(Debug, Deserialize)]
struct LicenseDescription {
    chosen_license: LicenseId,
    copyright_notice: String,
    link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseReport {
    package_name: String,
    conclusion: Result<LicenseDescription, IgnoredAny>,
}

impl quote::ToTokens for LicenseReport {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        let c: &LicenseDescription = self.conclusion.as_ref().unwrap();
        let (name, link, copyright, license) =
            (&self.package_name, &c.link, &c.copyright_notice, &c.chosen_license);

        let link = match link {
            &Some(ref link) => quote! { Some(#link) },
            &None => quote! { None },
        };

        tokens.append(quote! {
            LicenseInfo {
                name: #name,
                link: #link,
                copyright: #copyright,
                license: License::#license,
            }
        });
    }
}

pub fn licenses(_input: TokenStream) -> TokenStream {
    let mut license_infos = SOURCES
        .iter()
        .map(|x| -> Vec<LicenseReport> { serde_json::from_reader(File::open(x).unwrap()).unwrap() })
        .map(|x| x.into_iter().filter(|x| x.conclusion.is_ok()))
        .fold(vec![], |mut a, b| { a.extend(b); a });

    license_infos.sort_unstable_by_key(|x| x.package_name.to_lowercase());

    let gen = quote! {
        lazy_static! {
            static ref LICENSE_INFOS: &'static [LicenseInfo] = &[
                #(#license_infos,)*
            ];
        }
    };

    gen.parse().unwrap()
}
