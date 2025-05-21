# Crate: runtime
A binary uploaded on target machine to process complex tasks like extracting or running a deployed project

## Command Line Interface
- extract
  - extracts a compressed archive to destination
  - --archive: string
  - --destination: [string]
  - --overwrite: [bool] = false
- execute
  - executes a project with runtime specifications
  - --silent: [bool] = false
- build
  - builds a project from source