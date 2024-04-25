// @generated automatically by Diesel CLI.

diesel::table! {
    videos (id) {
        id -> BigInt,
        uid -> Text,
        link -> Nullable<Text>,
        title -> Nullable<Text>,
        author -> Nullable<Text>,
        duration -> Nullable<BigInt>,
        description -> Nullable<Text>,
    }
}
