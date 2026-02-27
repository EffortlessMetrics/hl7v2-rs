use hl7v2_core::Error;

use crate::merge::merge_profiles;
use crate::model::Profile;

/// Load profile from YAML
pub fn load_profile(yaml: &str) -> Result<Profile, Error> {
    serde_yaml::from_str(yaml).map_err(|_e| Error::InvalidEscapeToken) // TODO: Better error mapping
}

/// Load profile with inheritance resolution
///
/// This function loads a profile and recursively resolves any parent profiles,
/// merging their constraints and rules into a single profile.
///
/// # Arguments
///
/// * `yaml` - The YAML string for the profile
/// * `profile_loader` - A function that can load a parent profile by name
///
/// # Returns
///
/// A fully resolved profile with all inherited constraints merged
pub fn load_profile_with_inheritance<F>(yaml: &str, profile_loader: F) -> Result<Profile, Error>
where
    F: Fn(&str) -> Result<Profile, Error>,
{
    let profile = load_profile(yaml)?;

    // If there's a parent, recursively load and merge it
    if let Some(parent_name) = &profile.parent {
        let parent_profile = load_profile_with_inheritance_recursive(parent_name, &profile_loader)?;
        return Ok(merge_profiles(parent_profile, profile));
    }

    Ok(profile)
}

/// Recursively load parent profiles
fn load_profile_with_inheritance_recursive<F>(
    parent_name: &str,
    profile_loader: &F,
) -> Result<Profile, Error>
where
    F: Fn(&str) -> Result<Profile, Error>,
{
    let parent_profile = profile_loader(parent_name)?;

    // If the parent also has a parent, recursively load and merge it
    if let Some(grandparent_name) = &parent_profile.parent {
        let grandparent_profile =
            load_profile_with_inheritance_recursive(grandparent_name, profile_loader)?;
        return Ok(merge_profiles(grandparent_profile, parent_profile));
    }

    Ok(parent_profile)
}
