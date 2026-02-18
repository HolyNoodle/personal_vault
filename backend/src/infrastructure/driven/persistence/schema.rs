diesel::table! {
    users (id) {
        id -> Text,
        email -> Text,
        display_name -> Text,
        roles -> Text,
        status -> Text,
    }
}

diesel::table! {
    webauthn_credentials (id) {
        id -> Text,
        user_id -> Text,
        credential_id -> Text,
        public_key -> Text,
        sign_count -> BigInt,
        created_at -> Text,
    }
}
