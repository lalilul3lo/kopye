Todo:
- Conditional files and directories :: templating
- Config
    <!-- I should rename config to source -->
    - Default values (answers)
    - Default templates (like flake.nix)
- Integration tests
- Blueprints github repo (think about CI/CD pipeline) maybe I want to make sure all templates if binaries can run
  - example: https://github.com/superlinear-ai/substrate/blob/main/.github/workflows/test.yml

## Design
- inspo: https://www.makingsoftware.com/


## Impure 
I/O
 - Git clone repo
 - Mkdir
 - Create file and write
 - Prompting user

 Parsing Files
 - CLI configuration files
 
 Generating strings
 - tera render








 ## Error 

- Use try ? for propagating errors.
- Use exhaustive pattern matching on concerns you need to handle.
- Do not implement conversions from local concerns into global enums,
  or your local concerns will find themselves in inappropriate places over time.
- Using separate types will lock them out of where they don’t belong.
- Define errors in terms of the problem, not a solution

## Domains
- Config


## What is the purpose of...
- thiserror
- anyhow


##
As I begin writing domain specific errors, I'm noticing that I need to always map each error. Which forces me to address each error kind


## TEST Error
- pass repo that doesn't have config
- pass project that doesn't have a questions.toml

