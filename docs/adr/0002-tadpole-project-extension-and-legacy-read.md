# Standardize project files on `.tadpole` while keeping `.seocrawl` readable

Tadpole saves new Projects and registers OS file associations under the `.tadpole` extension to strengthen product branding, while keeping legacy `.seocrawl` files accepted in the Open dialog and CLI open handler. Both extensions share the same JSON Project schema, so dropping `.seocrawl` read support would break existing user archives for no technical gain.
