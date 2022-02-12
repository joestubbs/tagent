# tagent

![Lint, Test, and Build](../../actions/workflows/rust.yml/badge.svg)

`tagent` (Tapis Agent) is a light-weight webserver, written in Rust, and built using the actix-web framework.
`tagent` provides HTTP APIs for basic file management tasks. The server includes endpoints for listing file paths,
uploading and downloading files.

An OpenAPI v3 specification is included.

## Building the Project
The project requires a recent version of Rust (e.g., 1.57.0); install using `rustup`. With rust
installed, use `cargo` to build the project:

```
$ cargo build
```

or run a development server

```
$ cargo run
```

## Examples

The following examples use `curl` to illustrate the functionality.


1. List the files in the directory at the path `/rust` (relative to the app root_dir):

```
$ curl localhost:8080/files/list/rust | jq

{
  "message": "File listing retrieved successfully",
  "status": "success",
  "version": "0.1.0",
  "result": [
    "tmp",
    "oaicli",
    "web",
    "examples",
    "hello-rust",
    "docker-client",
    "testup"
  ]
}
```

2. List a specific file

```
$ curl localhost:8080/files/list/rust/tmp/testup.txt | jq
{
  "message": "File listing retrieved successfully",
  "status": "success",
  "version": "0.1.0",
  "result": [
    "/home/jstubbs/projects/rust/tmp/testup.txt"
  ]
}
```

3. Listing to a path that does not exist results in an error:

```
$ curl localhost:8080/files/list/rust/tmp/foo | jq
{
  "message": "Invalid path; path Some(\"/home/jstubbs/projects/rust/tmp/foo\") does not exist",
  "status": "error",
  "result": "none",
  "version": "0.1.0"
}
```

4. Upload a file called up.txt in the current working directory to `/rust/tmp`.

```
$ curl -F upload=@up.txt localhost:8080/files/contents/rust/tmp -v | jq
{
  "message": "file uploaded to Some(\"/home/jstubbs/projects/rust/tmp/up.txt\") successfully.",
  "status": "success",
  "version": "0.1.0",
  "result": "none"
}
```

5. The path specified in an upload request must be a directory; if it is not, an error is returned:

```
$ curl -F upload=@up.txt localhost:8080/files/contents/rust/tmp/up.txt -v | jq
{
  "message": "Invalid path; path \"/home/jstubbs/projects/rust/tmp/up.txt\" must be a directory",
  "status": "error",
  "result": "none",
  "version": "0.1.0"
}

