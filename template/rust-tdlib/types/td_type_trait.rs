{% set trait_name = token.name | to_camel %}
/// TRAIT | {{token.description}}
pub trait TD{{trait_name}}: Debug + RObject {}

/// {{token.description}}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag="@type")]
pub enum {{trait_name}} {
  #[doc(hidden)] _Default(()),
{% for subt in sub_tokens(token=token) %}  /// {{subt.description}}
  #[serde(rename(deserialize = "{{subt.name}}"))]
  {{subt.name | td_remove_prefix(prefix=trait_name) | to_camel}}({{subt.name | to_camel}}),
{% endfor %}
}

impl Default for {{trait_name}} {
  fn default() -> Self { {{trait_name}}::_Default(()) }
}

impl RObject for {{trait_name}} {
  #[doc(hidden)] fn extra(&self) -> Option<String> {
    match self {
{% for subt in sub_tokens(token=token) %}      {{trait_name}}::{{subt.name | td_remove_prefix(prefix=trait_name) | to_camel}}(t) => t.extra(),
{% endfor %}
      _ => None,
    }
  }
#[doc(hidden)] fn client_id(&self) -> Option<i32> {
    match self {
{% for subt in sub_tokens(token=token) %}      {{trait_name}}::{{subt.name | td_remove_prefix(prefix=trait_name) | to_camel}}(t) => t.client_id(),
{% endfor %}
      _ => None,
    }
  }
}

impl {{trait_name}} {
  pub fn from_json<S: AsRef<str>>(json: S) -> RTDResult<Self> { Ok(serde_json::from_str(json.as_ref())?) }
  #[doc(hidden)] pub fn _is_default(&self) -> bool { matches!(self, {{trait_name}}::_Default(_)) }
}

impl AsRef<{{trait_name}}> for {{trait_name}} {
  fn as_ref(&self) -> &{{trait_name}} { self }
}