In this repo we're trying again at the eternal struggle to make things convenient without letting them become too confusing.

In a situation where you've standardized on a set of deployable items that accept YAML configuration, life can be great until you're let down by inconsistent specifications (of YAML, for a start).

This repo is an attempt to alleviate the problem by generating configuration files in canonical JSON:

* It's still (perfectly) valid YAML
    * (although not all YAML is representable as valid JSON... we'll get to that)
    
* A specific configuration will always be represented in a repeatably identical way, so you can be sure if anything changes.

* JSON lets you know for sure if you're having the Norway problem (`Norway` ISO country code => `NO` => `false` in YAML).

* This project provides a simple templating system (substituting different values for placeholder variables, depending on the target "environment").
    * All the input to the templating system is YAML, for convenience.
        * Specifically it's `serde_yaml` from Rust, which generally omits YAML's most confusing features.
    * Plain text templates are also natively supported for output (with string-only templating, or optionally embedded JSON values).
    * Special filters, for extra processing of inputs or outputs, can be easily hacked in.
