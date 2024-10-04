Organizing configuration
------------------------

In this repo we're trying again at the eternal struggle to make things convenient without letting them become too confusing.

In a situation where you've standardized on a set of deployable items that accept YAML configuration, life can be great until you're let down by inconsistent specifications (of YAML, for a start).

This repo is an attempt to alleviate the problem by **generating configuration files** in **canonical JSON**:

* It's still (perfectly) valid YAML

* A specific configuration will always be represented in a repeatably identical way, so you can be sure if anything changes.

* JSON lets you know for sure if you're having the Norway problem (`Norway` ISO country code => `NO` => `false` in YAML).

> [!NOTE]
> Not all possible YAML is representable as valid JSON, for example JSON keys must always be strings. Most systems only need JSON in practice, at least.

What is this, then?
-------------------

This project provides a simple templating system (substituting different values for placeholder variables, depending on the target "environment").

* All the input to the templating system is YAML, for convenience.    
        * Specifically it's parsed by `serde_yaml` from Rust, which generally omits YAML's most confusing optional/deprecated features.
  
* Plain text templates are also natively supported for output (with string-only substitution, or optionally, embedded canonical JSON values).

* Special filters, for extra processing of inputs or outputs, can be easily hacked in.


History
-------

This started off as a sort of ad-hoc extensible templating system in Ruby, inside a project config repo.

The idea was based on good and bad experiences with Hiera (within Puppet) and tools like that. The idea was to make things simpler by keeping things close to being static (as much as possible) so that anyone could trivially see for sure what configuration each environment would have when a particular version of the repo was deployed.

The switch to a statically compiled tool interpreting a fixed specification was a move to not only improve performance in CI but also safety as part of a production deployment system, as the embedded Ruby version was inevitably open to potential code injection (albeit not too much of a concern within an environment with supposedly trusted employees). A similar idea also seems to have influenced [dhall](https://dhall-lang.org/) (a specific sandboxed and limited language for expressing configuration), although the design tradeoffs are otherwise quite different -- it allows for more complexity than this tool, and doesn't directly suit this simple "templating" flavour.

I believe the simplicity of the approach here is its main strength -- it's easy to start with a bunch of static files and then begin to introduce templating as needed for different deployments.


Testing
-------

This repo is somewhat light on testing as the implementation was initially tested in context with the real configurations, simply by checking whether its output agreed with the prototype Ruby implementation. The major benefit ef using canonical JSON -- the elimination of trivial formatting differences -- is useful in this situation as in many others.

As Ruby's Psych YAML parser implements some semi-deprecated features from different versions of YAML that Rust's serde_yaml does not, this had the pleasant side effect of also failing where ambiguous forms of input were used.

This tool in itself is pretty simple, so there's not much to really go wrong. In any case, the question is whether it does what you expect. The main way to gain confidence is to just validate that the output you see is the output you wanted. Nonetheless I may put in some sample-based tests fairly soon.

Another highly useful tool for day-to-day testing could be a [`configuration-diff` wrapper](https://gist.github.com/mehhhhhhhhhhhhhhh/6ddedbacaf69ab6b2117abb2b297933c#file-config-diff-rb) which acts a lot like `git diff` except it shows the changes in the compiled config, not in the source. This is why this tool includes the option of directly "pretty-printing" YAML output instead of canonicalizing it -- it just makes it easier and quicker to get a nice readable diff for this purpose.
