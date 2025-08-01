# Environment Variables

The Office Reader MCP server supports environment variables to configure its behavior.

## PROJECT_ROOT

The `PROJECT_ROOT` environment variable is used to resolve relative file paths and provides security restrictions when configured.

### Usage

Set the `PROJECT_ROOT` environment variable to the root directory of your project:

```bash
# Example: Set PROJECT_ROOT to your project directory
export PROJECT_ROOT="/path/to/your/project"
```

### Security Model

The server implements different security behaviors based on whether `PROJECT_ROOT` is configured:

#### When PROJECT_ROOT is NOT set:
- Both absolute and relative file paths are allowed
- Relative paths are resolved against the current working directory
- No security restrictions are applied

#### When PROJECT_ROOT IS set:
- **Absolute paths are REJECTED** for security reasons
- Only relative paths are allowed
- All relative paths are resolved against the `PROJECT_ROOT` directory
- The `PROJECT_ROOT` directory must exist

### How it works

When you provide a file path to any of the MCP tools, the server will:

1. Check if `PROJECT_ROOT` is configured
2. If `PROJECT_ROOT` is set:
   - **Reject absolute paths** with an error message
   - Resolve relative paths against `PROJECT_ROOT`
   - Verify that `PROJECT_ROOT` directory exists
3. If `PROJECT_ROOT` is not set:
   - Allow both absolute and relative paths
   - Resolve relative paths against the current working directory

### Examples

#### When PROJECT_ROOT is NOT set:
- Relative path: `documents/report.pdf` → Resolved to: `<current_dir>/documents/report.pdf`
- Absolute path: `/tmp/file.xlsx` → Used as-is: `/tmp/file.xlsx`

#### When PROJECT_ROOT="/home/user/myproject":
- Relative path: `documents/report.pdf` → Resolved to: `/home/user/myproject/documents/report.pdf`
- Absolute path: `/tmp/file.xlsx` → **REJECTED** with error: "Absolute paths are not allowed when PROJECT_ROOT is configured for security reasons"

### MCP Tools Affected

All document processing tools support relative paths:

- `get_document_page_info`
- `read_office_document`
- `read_powerpoint_slides`
- `get_powerpoint_slide_info`
- `generate_powerpoint_slide_snapshot`
- `stream_office_document`

### Tool Descriptions

All tool parameter descriptions now indicate support for both absolute and relative paths:
> Path to the office document file (absolute or relative to PROJECT_ROOT environment variable)

## Notes

- **Security**: When `PROJECT_ROOT` is configured, absolute paths are completely blocked to prevent access to files outside the project directory
- If a relative path is provided and neither `PROJECT_ROOT` nor current working directory resolution works, the tool will return a "file not found" error
- The environment variable is checked each time a file path needs to be resolved
- Changes to `PROJECT_ROOT` take effect immediately for new requests
- Path resolution happens at the MCP tool entry points for optimal performance and security