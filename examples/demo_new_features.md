# Office Reader MCP - New Features Demo

This document demonstrates the new features added to the Office Reader MCP server:

1. **Text Length Checking** - Get document size without reading full content
2. **Pagination Support** - Read documents in chunks with offset and size limits
3. **Enhanced Error Handling** - Better feedback for large document processing

## Feature 1: Text Length Checking

### Tool: `get_document_text_length`

**Purpose**: Check the total text length of a document before processing it.

**Example Request**:
```json
{
  "name": "get_document_text_length",
  "arguments": {
    "file_path": "/path/to/document.pdf"
  }
}
```

**Example Response**:
```json
{
  "file_path": "/path/to/document.pdf",
  "total_length": 125000,
  "file_exists": true,
  "error": null
}
```

**Use Cases**:
- Determine if a document is too large for single-pass processing
- Estimate processing time
- Choose between streaming vs. pagination approaches
- Display file size information to users

## Feature 2: Pagination Support

### Tool: `read_office_document` (Enhanced)

**Purpose**: Read documents in manageable chunks with offset and size controls.

**New Parameters**:
- `max_size` (optional): Maximum characters to return (default: 50,000)
- `offset` (optional): Character offset to start reading from (default: 0)

### Example: Reading a Large Document in Chunks

#### Step 1: Check document size
```json
{
  "name": "get_document_text_length",
  "arguments": {
    "file_path": "/path/to/large-report.pdf"
  }
}
```

**Response**:
```json
{
  "file_path": "/path/to/large-report.pdf",
  "total_length": 150000,
  "file_exists": true,
  "error": null
}
```

#### Step 2: Read first chunk
```json
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/large-report.pdf",
    "max_size": 30000,
    "offset": 0
  }
}
```

**Response**:
```json
{
  "file_path": "/path/to/large-report.pdf",
  "total_length": 150000,
  "offset": 0,
  "returned_length": 30000,
  "has_more": true,
  "content": "# Large Report\n\n## Executive Summary\n\nThis report covers..."
}
```

#### Step 3: Read second chunk
```json
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/large-report.pdf",
    "max_size": 30000,
    "offset": 30000
  }
}
```

**Response**:
```json
{
  "file_path": "/path/to/large-report.pdf",
  "total_length": 150000,
  "offset": 30000,
  "returned_length": 30000,
  "has_more": true,
  "content": "## Chapter 2: Methodology\n\nOur approach involved..."
}
```

#### Step 4: Continue until complete
```json
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/large-report.pdf",
    "max_size": 30000,
    "offset": 120000
  }
}
```

**Response**:
```json
{
  "file_path": "/path/to/large-report.pdf",
  "total_length": 150000,
  "offset": 120000,
  "returned_length": 30000,
  "has_more": false,
  "content": "## Conclusion\n\nIn summary, our findings indicate..."
}
```

## Feature 3: Backward Compatibility

### Tool: `read_office_document_legacy`

**Purpose**: Maintain compatibility with existing implementations that expect full document content.

**Example Request**:
```json
{
  "name": "read_office_document_legacy",
  "arguments": {
    "file_path": "/path/to/small-document.xlsx"
  }
}
```

**Response**:
```json
{
  "content": "# Spreadsheet\n\n## Sheet: Data\n\n| Name | Value |\n|------|-------|\n| Item1 | 100 |\n| Item2 | 200 |"
}
```

## Recommended Usage Patterns

### Pattern 1: Smart Document Processing

```javascript
async function processDocument(filePath) {
  // Step 1: Check document size
  const lengthInfo = await mcp.call("get_document_text_length", { file_path: filePath });
  
  if (!lengthInfo.file_exists) {
    throw new Error(`File not found: ${filePath}`);
  }
  
  if (lengthInfo.error) {
    throw new Error(`Error reading file: ${lengthInfo.error}`);
  }
  
  // Step 2: Choose processing strategy based on size
  if (lengthInfo.total_length <= 50000) {
    // Small document - read all at once
    return await mcp.call("read_office_document", { file_path: filePath });
  } else {
    // Large document - read in chunks
    return await processLargeDocument(filePath, lengthInfo.total_length);
  }
}

async function processLargeDocument(filePath, totalLength) {
  const chunkSize = 25000;
  const chunks = [];
  let offset = 0;
  
  while (offset < totalLength) {
    const chunk = await mcp.call("read_office_document", {
      file_path: filePath,
      max_size: chunkSize,
      offset: offset
    });
    
    chunks.push(chunk.content);
    offset += chunk.returned_length;
    
    // Progress feedback
    const progress = (offset / totalLength * 100).toFixed(1);
    console.log(`Progress: ${progress}% (${offset}/${totalLength} characters)`);
    
    if (!chunk.has_more) break;
  }
  
  return chunks.join('\n');
}
```

### Pattern 2: Progressive Loading

```javascript
async function* loadDocumentProgressively(filePath, chunkSize = 30000) {
  const lengthInfo = await mcp.call("get_document_text_length", { file_path: filePath });
  
  if (!lengthInfo.file_exists || lengthInfo.error) {
    throw new Error(lengthInfo.error || "File not found");
  }
  
  let offset = 0;
  const totalLength = lengthInfo.total_length;
  
  while (offset < totalLength) {
    const chunk = await mcp.call("read_office_document", {
      file_path: filePath,
      max_size: chunkSize,
      offset: offset
    });
    
    yield {
      content: chunk.content,
      progress: {
        current: offset + chunk.returned_length,
        total: totalLength,
        percentage: ((offset + chunk.returned_length) / totalLength * 100).toFixed(1),
        hasMore: chunk.has_more
      }
    };
    
    offset += chunk.returned_length;
    if (!chunk.has_more) break;
  }
}

// Usage
for await (const chunk of loadDocumentProgressively("/path/to/large-document.pdf")) {
  console.log(`Loaded ${chunk.progress.percentage}% - ${chunk.content.substring(0, 100)}...`);
  // Process chunk.content
}
```

## Error Handling Examples

### File Not Found
```json
{
  "file_path": "/nonexistent/file.pdf",
  "total_length": 0,
  "file_exists": false,
  "error": null
}
```

### Unsupported File Type
```json
{
  "file_path": "/path/to/file.txt",
  "total_length": 0,
  "file_exists": true,
  "error": "Unsupported file type: txt"
}
```

### Processing Error
```json
{
  "file_path": "/path/to/corrupted.pdf",
  "total_length": 0,
  "file_exists": true,
  "error": "Error reading PDF file: Failed to extract text from PDF"
}
```

## Performance Benefits

### Before (Single Tool)
- ❌ Large files could timeout
- ❌ No size estimation
- ❌ All-or-nothing processing
- ❌ Memory intensive for huge documents

### After (Enhanced Tools)
- ✅ Size checking before processing
- ✅ Configurable chunk sizes (default: 50,000 chars)
- ✅ Pagination support with offset
- ✅ Memory efficient for large documents
- ✅ Progress tracking and metadata
- ✅ Backward compatibility maintained

## Migration Guide

### From Old API
```json
// Old way
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/document.pdf"
  }
}
```

### To New API (Recommended)
```json
// Step 1: Check size
{
  "name": "get_document_text_length",
  "arguments": {
    "file_path": "/path/to/document.pdf"
  }
}

// Step 2: Read with limits (if needed)
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/document.pdf",
    "max_size": 50000,
    "offset": 0
  }
}
```

### For Backward Compatibility
```json
// Use legacy tool for existing code
{
  "name": "read_office_document_legacy",
  "arguments": {
    "file_path": "/path/to/document.pdf"
  }
}
``` 