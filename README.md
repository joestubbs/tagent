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


### Working with Files

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

### Working with ACLs

``tagent`` can utilize an authorization system based on ACLs (Access Control List)s. 
Each ACL specifies whether an authenticated subject is authorized (decision: ``Allow``) or not authorized (decision: ``Deny``)
to perform certain requests. There are 5 aspects to an ACL:

  * ``subject`` -- The subject of the ACL. This must be an exact match to the subject making the request for the
  * ``action`` -- The action being taken. This can be one of ``Read``, ``Execute`` or ``Write``. The actions are ordered:
     ``Read`` is less than ``Execute`` and ``Execute`` is less than ``Write``. 
  * ``path`` -- The URL path associated with the ACL. This can be a string literal or it can contain a regular expression.
  * ``user`` -- The user that the subject is acting on behalf of, or ``self`` when the subject is acting as itself.
  * ``decision`` -- Whether the ACL authorizes (``Allow``) or does not authorize (``Deny``) the subject for the request.

When checking ACLs, ``tagent`` uses the following algorithm:

  1. If any ACL with decision ``Deny`` matches the request, the request is not authorized, 
  2. Otherwise, if any ACL with decision ``Allow`` matches the request, the request is authorized,
  3. Otherwise, no ACLs match the request, and the request is not authorized.

  NOTE: 3) uses a "default decision", which could be configurable in a future version.

Because ``tagent`` checks ``Deny`` ACLs first, it is possible for certain ``Allow`` ACLs to be "superfluous"; i.e., they
do not impact the permissions decisions of ``tagent`` because they are eclipsed by ``Deny`` decisions. Currently, ``tagent``
does not detect such instances, but in a future version it will. 

Examples

1. Create a new ACL giving write access to the ``/tmp/testup.txt`` path.

```
$ curl -H "content-type: application/json" -d '{"subject": "tenants@admin", "action": "Write", "user": "self", "path": "`/tmp/testup.txt", "decision": "Allow"}'  -H "x-tapis-token: $jwt" localhost:8080/acls |jq

{
  "message": "ACL for tenants@admin created successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

2. Create an ``Allow`` ACL with a wild card that matches any files in the root directory with an extension of ``.txt``. Note that 
we use a regular expression syntax here, where the ".*" matches any characters.
```
$ curl -H "content-type: application/json" -d '{"subject": "tenants@admin", "action": "Write", "user": "self", "path": "/.*.txt", "decision": "Allow"}'  -H "x-tapis-token: $jwt" localhost:8080/acls|jq

{
  "message": "ACL for tenants@admin created successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

3. Create a ``Deny`` ACL that prevents read access to any file that starts with a name that starts with ``exam`` in the root 
directory. Again, we use a regular expression syntax. 

```
$ curl -H "content-type: application/json" -d '{"subject": "tenants@admin", "action": "Read", "user": "self", "path": "/exam.*", "decision": "Deny"}'  -H "x-tapis-token: $jwt" localhost:8080/acls |jq

{
  "message": "ACL for subject tenants@admin created successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```

4. We can list all ACLs in the system. Each ACL was assigned a unique id.

```
$ curl  -H "x-tapis-token: $jwt" localhost:8080/acls

{
  "message": "ACLs retrieved successfully.",
  "status": "success",
  "version": "0.1.0",
  "result": [
    {
      "id": 3,
      "subject": "tenants@admin",
      "action": "Write",
      "path": "/tmp/testup.txt",
      "user": "self",
      "decision": "Allow",
      "create_by": "tenants@admin",
      "create_time": "2022-02-25T02:22:28.537654901+00:00"
    },
    {
      "id": 4,
      "subject": "tenants@admin",
      "action": "Read",
      "path": "/exam.*",
      "user": "self",
      "decision": "Deny",
      "create_by": "tenants@admin",
      "create_time": "2022-02-26T21:00:06.604483349+00:00"
    },
    {
      "id": 6,
      "subject": "tenants@admin",
      "action": "Write",
      "path": "/.*.txt",
      "user": "self",
      "decision": "Allow",
      "create_by": "tenants@admin",
      "create_time": "2022-02-26T21:30:19.907017671+00:00"
    }
  ]
}
```

5. We can use the id's to retrieve, update or delete a specific ACL:

```
$ curl  -H "x-tapis-token: $jwt" localhost:8080/acls/3
{
  "message": "ACL retrieved successfully.",
  "status": "success",
  "version": "0.1.0",
  "result": {
    "id": 3,
    "subject": "tenants@admin",
    "action": "Write",
    "path": "/tmp/testup.txt",
    "user": "self",
    "decision": "Allow",
    "create_by": "tenants@admin",
    "create_time": "2022-02-25T02:22:28.537654901+00:00"
  }
}
```

We can ask ``tagent`` if a specific request will be authorized by providing a subject, user, action and path to the
``/acl/isauthz`` endpoint.

6. We authorized ``tenants@admin`` for the path ``/tmp/testup.txt`` explicitly, so we expect a ``true`` response to 
the following request:

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/iauthz/tenants@admin/self/Read/tmp/testup.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "true",
  "version": "0.1.0"
}

```


7. We also authorized ``tenants@admin`` for any path ending in a ``.txt`` extension in the root at the ``Write`` level, so the 
following also all return ``true`` responses: 

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Write/testup.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "true",
  "version": "0.1.0"
}


$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Execute/aa123.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "true",
  "version": "0.1.0"
}

```

Note that subdirectories match our regular expression, so this request also returns ``true``

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Execute/foo/bar/aa123.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "true",
  "version": "0.1.0"
}
```

8. However, we explicitly created a ``Deny`` ACL for all paths starting with ``exam`` in the root (at a ``Read`` level), so
the following return ``false``:

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Execute/exam123.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "false",
  "version": "0.1.0"
}

$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Read/examA.txt

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "false",
  "version": "0.1.0"
}

```

9. If we try a path not explicitly covered by the ACLs, we get the default decision (``false``):

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/isauthz/tenants@admin/self/Read/test.zip

{
  "message": "Result of authz check returned",
  "status": "success",
  "result": "false",
  "version": "0.1.0"
}

```

10. An example of updating an ACL by id:

```
$ curl -H "x-tapis-token: $jwt" localhost:8080/acls/3 -X PUT -H "content-type: application/json" -d '{"subject": "jobs@admin", "action": "Write", "path": "/*", "user": "self", "decision": "Allow"}'

{
  "message": "ACL updated successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```



11. We can delete an ACL by id
```
curl -H "x-tapis-token: $jwt" localhost:8080/acls/3 -X DELETE

{
  "message": "ACL deleted successfully.",
  "status": "success",
  "result": "none",
  "version": "0.1.0"
}
```



