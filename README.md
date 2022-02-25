# tagent

![Lint, Test, and Build](../../actions/workflows/rust.yml/badge.svg)

`tagent` (Tapis Agent) is a light-weight webserver, written in Rust, and built using the actix-web framework.
`tagent` provides HTTP APIs for basic file management tasks. The server includes endpoints for listing file paths,
uploading and downloading files.

An OpenAPI v3 specification is included.

## Building the Project
The project requires a recent version of Rust (e.g., 1.57.0); install using `rustup`. 
You also need OpenSSL (both the library and headers) required for building the rust-openssl
lib. (See https://docs.rs/crate/openssl-sys/0.9.36). On Debian/Ubuntu, you can install with

```
$ sudo apt-get install pkg-config libssl-dev
```

### DB Setup Prequisites 

For working with sqlite, you will need the sqlite libraries. On Debian/Ubuntu: 

```
$ sudo apt-get install sqlite3 libsqlite3-dev
```

and the Diesel CLI:

```
$ cargo install diesel_cli --no-default-features --features sqlite
```

Within the `tagent/tagent` directory, run the following commands to set up the database. 

1) Create the database:

```
$ diesel setup
```

2) Run migrations:

```
$ diesel migration run
```

### Compile tagent

With rust and the pre-requisites installed, use `cargo` to build the project:

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
```

6. Create a new ACL 

```
$ curl -H "content-type: application/json" -d '{"subject": "tenants@admin", "action": "Write", "user": "self", "path": "/*"}'  -H "x-tapis-token: $jwt" localhost:8080/acls |jq

{
  "message": "ACL created successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

7. List all acls

```
$ curl  -H "x-tapis-token: $jwt" localhost:8080/acls

{
  "message": "ACLs retrieved successfully.",
  "status": "success",
  "version": "0.1.0",
  "result": [
    {
      "id": 1,
      "subject": "tenants@admin",
      "action": "Write",
      "path": "/*",
      "user": "self",
      "decision": "Allow",
      "create_by": "tenants@admin",
      "create_time": "2022-02-23T04:59:53.721885536+00:00"
    },
    {
      "id": 2,
      "subject": "files@admin",
      "action": "Write",
      "path": "/files/*",
      "user": "self",
      "decision": "Allow",
      "create_by": "tenants@admin",
      "create_time": "2022-02-23T05:02:49.315917222+00:00"
    },
  ]
}
```

8. Retrieve an ACL by id

```
$ curl  -H "x-tapis-token: $jwt" localhost:8080/acls/2
{
  "message": "ACL retrieved successfully.",
  "status": "success",
  "version": "0.1.0",
  "result": {
    "id": 2,
    "subject": "files@admin",
    "action": "Write",
    "path": "/files/*",
    "user": "self",
    "decision": "Allow",
    "create_by": "tenants@admin",
    "create_time": "2022-02-23T05:02:49.315917222+00:00"
  }
}
```

9. Delete an ACL by id:

```
$ curl  -H "x-tapis-token: $jwt" localhost:8080/acls/2 -X DELETE

{
  "message": "ACL deleted successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

10. Update an ACL by id:

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/3 -X PUT -H "content-type: application/json" -d '{"subject": "jobs@admin", "action": "Write", "path": "/*", "user": "self", "decision": "Allow"}'

{
  "message": "ACL updated successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

