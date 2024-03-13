// @generated automatically by Diesel CLI.

diesel::table! {
    videos (pk) {
        pk -> Integer,
        uid -> Nullable<Text>,
        link -> Nullable<Text>,
        title -> Nullable<Text>,
        author -> Nullable<Text>,
        duration -> Nullable<Integer>,
        description -> Nullable<Text>,
        thumbnail_path -> Nullable<Text>,
        date -> Nullable<Integer>,
        other -> Nullable<Binary>,
    }
}
