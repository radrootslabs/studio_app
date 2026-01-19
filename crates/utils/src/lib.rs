#![forbid(unsafe_code)]

pub mod error;
pub mod errors;
pub mod r#async;
pub mod binary;
pub mod cache;
pub mod currency;
pub mod id;
pub mod media;
pub mod model;
pub mod numbers;
pub mod object;
pub mod path;
pub mod text;
pub mod time;
pub mod types;
pub mod unit;
pub mod validation;

pub use r#async::exe_iter;
pub use binary::{as_array_buffer, RadrootsAppArrayBuffer};
pub use cache::{
    asset_cache_fetch, asset_cache_fetch_bytes, AssetBytes, AssetCacheFetchConfig, AssetCacheMode,
    AssetCacheRequestInit, AssetResponse, RADROOTS_ASSET_CACHE_NAME, RADROOTS_ASSET_CACHE_PREFIX,
};
pub use currency::{
    fmt_price, parse_currency, parse_currency_marker, price_to_formatted, FiatCurrency,
    FIAT_CURRENCIES,
};
pub use id::{d_tag_create, uuidv4, uuidv4_b64url, uuidv7, uuidv7_b64url};
pub use media::{fmt_media_image_upload_result_url, MediaImageUploadResult, MediaResource};
pub use model::{
    is_model_query_filter_option, is_model_query_filter_option_list, is_model_query_values,
    list_model_query_values_assert, parse_model_query_value, ModelForm, ModelFormErrorTuple,
    ModelFormValidationTuple, ModelQueryBindValue, ModelQueryBindValueOpt, ModelQueryBindValueTuple,
    ModelQueryFilterCondition, ModelQueryFilterOption, ModelQueryFilterOptionList,
    ModelQueryParam, ModelQueryValue, ModelSchemaErrors, ModelSortCreatedAt,
};
pub use errors::{err_msg, handle_err, throw_err, ERR_PREFIX_APP, ERR_PREFIX_UTILS};
pub use numbers::{num_interval_range, num_str, parse_float, parse_int};
pub use object::{obj_en, obj_result, obj_results_str, obj_truthy_fields};
pub use path::{
    parse_route_path,
    resolve_route_path,
    resolve_wasm_path,
    RadrootsAppRoutePathParts,
};
pub use text::{str_cap, str_cap_words, text_dec, text_enc, ROOT_SYMBOL};
pub use time::{time_now_ms, time_now_s};
pub use types::{
    resolve_err, resolve_ok, FileBytesFormat, FilePath, FilePathBlob, FileMimeType, IdbClientConfig,
    ResolveError, ResultBool, ResultId, ResultObj, ResultPass, ResultPublicKey, ResultSecretKey,
    ResultsList, ValidationRegex, ValStr, WebFilePath,
};
pub use unit::{
    mass_to_g, parse_area_unit, parse_area_unit_default, parse_mass_unit, parse_mass_unit_default,
    AreaUnit, MassUnit, AREA_UNITS, MASS_UNITS,
};
pub use validation::regex::{form_fields, FormField, FormFieldsKey, UtilRegex};
pub use validation::schema::{
    zf_area_unit, zf_email, zf_mass_unit, zf_numf_pos, zf_numi_pos, zf_price, zf_price_amount,
    zf_quantity_amount, zf_username,
};
