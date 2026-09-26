use crate::{
    auth::get_user_1,
    create_cook_and_run,
    email::{
        create_self_service_team_with_mail, message_body, set_plan_config_language,
        unique_test_email, wait_for_message_to,
    },
    sharing::post_test::create_share_config_default,
};

fn invitation_body_for(cook_and_run_id: &uuid::Uuid, label: &str) -> String {
    let mail = unique_test_email(label);
    create_self_service_team_with_mail(cook_and_run_id, &mail);
    message_body(&wait_for_message_to(&mail, 15))
}

/// "language is resolved from plan_config.language (Deutsch -> de,
/// English -> en)" (§4.8). We don't know the actual translated strings
/// (not provided outside the — not shared — template files), so this
/// doesn't assert on specific words; it confirms the two configured
/// languages actually produce *different* rendered output for otherwise
/// identical teams, which is the part we can verify without guessing.
#[test]
fn test_invitation_email_language_differs_between_deutsch_and_english() {
    let cook_and_run_de = create_cook_and_run();
    let (admin_token, _) = get_user_1();
    create_share_config_default(
        &cook_and_run_de,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );
    set_plan_config_language(&cook_and_run_de, "Deutsch");
    let body_de = invitation_body_for(&cook_and_run_de, "lang-de");

    let cook_and_run_en = create_cook_and_run();
    create_share_config_default(
        &cook_and_run_en,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );
    set_plan_config_language(&cook_and_run_en, "English");
    let body_en = invitation_body_for(&cook_and_run_en, "lang-en");

    assert_ne!(
        body_de, body_en,
        "Deutsch- and English-configured projects should render different invitation emails"
    );
}

/// "... defaulting to de if no plan_config exists yet for the project."
/// (§4.8) This one *is* a precise, non-speculative check: a project with
/// no plan_config at all should render byte-identical to one explicitly
/// set to "Deutsch" (same template, same resolved language), given
/// otherwise-identical inputs.
#[test]
fn test_invitation_email_defaults_to_german_without_plan_config() {
    let (admin_token, _) = get_user_1();

    let cook_and_run_explicit_de = create_cook_and_run();
    create_share_config_default(
        &cook_and_run_explicit_de,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );
    set_plan_config_language(&cook_and_run_explicit_de, "Deutsch");
    let body_explicit_de = invitation_body_for(&cook_and_run_explicit_de, "lang-explicit-de");

    let cook_and_run_no_config = create_cook_and_run();
    create_share_config_default(
        &cook_and_run_no_config,
        &admin_token,
        false,
        false,
        &vec![],
        &None,
        &None,
    );
    // No plan_config PATCH at all for this project.
    let body_default = invitation_body_for(&cook_and_run_no_config, "lang-default");

    assert_eq!(
        body_explicit_de, body_default,
        "with no plan_config, the invitation email should render exactly as if language=Deutsch"
    );
}

// Note: `plan_config::patch_test::get_plan_config_patch_json` (the
// pre-existing fixture used by every other plan_config test) sends
// `"language": "eng"` — which is neither "Deutsch" nor "English", the two
// values this feature actually recognizes per §4.8. Since plan_config
// wasn't part of the v0.2.0 DTO changes, we didn't touch that fixture (it
// only tests that the field round-trips, not that "eng" is a recognized
// value) — flagging it here since it's directly relevant to this file and
// worth reconciling with whoever owns the plan_config language field.
