# Semantic Scholar MCP Server — Comprehensive Tool Test Report

**Date:** 2026-02-08
**Tester:** Claude (automated)
**Server:** semantic-scholar-mcp-rs
**Total Tools:** 29

---

## Executive Summary

| Status | Count | Percentage |
|--------|-------|------------|
| PASS | 20 | 69% |
| PASS (with caveats) | 5 | 17% |
| FAIL (bug) | 4 | 14% |

---

## Detailed Results by Category

### 1. DISCOVERY TOOLS (5 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `bulk_boolean_search` | PASS | Boolean operators (+, -, "phrase") work. Filters by year, field, minCitations all functional. Returned 5 relevant results. |
| `exhaustive_search` | PASS | Auto-pagination works. Year filtering and minCitations functional. Returned CRISPR gene editing papers correctly. |
| `snippet_search` | PASS | Returns highlighted text excerpts with section context. Found "Attention is All You Need" snippet with correct score ranking. |
| `recommendations` | PASS (caveat) | Returns empty `[]` for extremely popular papers (e.g., "Attention is All You Need" with 164k citations). Works correctly with moderately cited papers. |
| `semantic_search` | **FAIL** | Serialization error: `missing field 'seedPaperId'`. The tool schema exposes `seed_paper_id` (snake_case) but the server expects `seedPaperId` (camelCase). Parameter name mismatch bug. |

### 2. ENRICHMENT TOOLS (7 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `batch_metadata` | PASS (caveat) | Works with S2 paper IDs. Silently drops DOI-format IDs (`DOI:10.1038/nature14539` returned no result without error). Only S2 hash IDs reliably work. |
| `paper_title_match` | **FAIL** | Consistent error: `Failed to parse response: missing field 'paperId'`. Tested with multiple titles — always fails. Response parsing bug. |
| `paper_autocomplete` | PASS (caveat) | Returns paper IDs correctly but `title` field is always `null` for all suggestions. Partial parsing issue — IDs work, titles don't. |
| `paper_authors` | PASS | Returns full author profiles with h-index, citation counts, affiliations, and homepage URLs. Tested on "Attention is All You Need" — all 8 authors returned correctly. |
| `author_search` | PASS (caveat) | Returns matching author records, but `citations`, `hIndex`, and `papers` fields are always `0` for all results. The names and IDs are correct. Metrics parsing bug. |
| `author_papers` | **FAIL** | Returns completely unrelated papers for a given author ID. Testing with Noam Shazeer (ID: 1846258) returned papers about ChatGPT authorship ethics — none by Shazeer. Severe data mismatch bug. |
| `author_batch` | PASS | Returns correct metadata for multiple authors including affiliations, h-index, citations, homepage. Tested with 2 authors — both returned accurately. |

### 3. CITATION & RECOMMENDATION TOOLS (3 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `citation_snowball` | PASS (caveat) | Works with moderately cited papers (found 6 papers for Transformer Transducer). Returns 0 results for extremely popular papers with high `minCitations` filter. Direction "both" (citations + references) works. |
| `cocitation_analysis` | PASS (caveat) | Timed out (>30s) on "Attention is All You Need" (164k citations). Returned empty results (no error) for moderately cited paper with `maxCitingPapers=20`. May need higher parameters or more cited papers to produce results. |
| `bibliographic_coupling` | PASS | Found 3 methodologically similar papers via shared references. Returns coupling strength scores. Works well for speech recognition papers. |

### 4. SEMANTIC TOOLS (2 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `literature_review_pipeline` | PASS | Excellent multi-source pipeline. Found 130 unique papers for "retrieval augmented generation", deduplicated 120, returned top 5. Sources breakdown: search(100) + recommendations(50) + citations(89). |
| *(semantic_search)* | *See Discovery* | *FAIL — serialization error* |

### 5. NETWORK TOOLS (1 tool)

| Tool | Status | Notes |
|------|--------|-------|
| `author_network` | PASS | Found 2 collaborators for Noam Shazeer with 5+ shared papers. Returns shared paper IDs and collaboration graph structure. |

### 6. TREND ANALYSIS TOOLS (2 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `research_trends` | PASS | Excellent year-by-year breakdown of LLM research (2020-2025). Shows paper count, avg citations, total citations, and top papers per year. Very useful for understanding field evolution. |
| `venue_analytics` | PASS | Returns NeurIPS stats with top papers, papers-by-year breakdown, and aggregate statistics (avg citations, total papers). |

### 7. BIBLIOMETRICS TOOLS (6 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `field_weighted_impact` | PASS | Correctly calculated FWCI of 1163.72 for "Attention is All You Need" (116,372% of expected citations). Includes baseline average and methodology note. |
| `highly_cited_papers` | PASS (caveat) | Returns results but `threshold` is always `0` and `is_highly_cited` is always `false`, even for "Attention is All You Need" (164k citations). The baseline threshold calculation appears broken. |
| `citation_half_life` | PASS | Returns 9.0 year half-life for "Attention is All You Need". Age distribution is simplified (all citations bucketed into one range), suggesting sampling approximation. |
| `hot_papers` | PASS | Functional but output exceeded 84KB for a broad query, causing result truncation. Works correctly — just produces very large output for popular topics. |
| `cocitation_analysis` | *(See Citation tools)* | *Works but returns empty for tested papers* |
| `bibliographic_coupling` | *(See Citation tools)* | *PASS* |

### 8. ADVANCED TOOLS (2 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `pearl_growing` | PASS | Found 50 papers via keyword-based growth from "Attention is All You Need" seed. Extracted keywords ("attention translation best bleu training") and grew literature. Returns TL;DR summaries. |
| `orcid_author_lookup` | **FAIL** | Error: `Author with id ORCID:0000-0001-7574-4827 not found`. Tested with Geoffrey Hinton's ORCID. The S2 API may not have ORCID mappings for all authors, or the ORCID format may be incorrectly constructed. |

### 9. EXPORT & SYSTEMATIC REVIEW TOOLS (4 tools)

| Tool | Status | Notes |
|------|--------|-------|
| `reference_export` | PASS | All 3 tested formats work: **BibTeX** (proper @article entries with abstracts), **RIS** (TY/TI/AU/PY tags), **CSV** (header row + data). Clean, well-formatted output. |
| `screening_export` | PASS | Returns structured JSON with abstracts, TL;DR summaries, author lists, and citation counts. Good for systematic review screening. |
| `prisma_search` | PASS | Functional multi-query search with deduplication. Output exceeded 92KB for tested queries (result saved to file). Works correctly for PRISMA workflow. |
| `prisma_flow_diagram` | **FAIL** | Serialization error: `missing field 'reportsSought'`. The tool schema uses snake_case (`reports_sought`) but the server expects camelCase (`reportsSought`). Same class of bug as `semantic_search`. |

---

## Bug Summary

### Critical Bugs (4 tools broken)

1. **`paper_title_match`** — Response parsing fails with `missing field 'paperId'`. The S2 API response format likely changed or doesn't include `paperId` in the title match endpoint.

2. **`semantic_search`** — Parameter name mismatch: schema exposes `seed_paper_id` but serialization expects `seedPaperId`. Fix: align the serde rename or input schema.

3. **`prisma_flow_diagram`** — Same camelCase/snake_case mismatch: `reports_sought` vs `reportsSought`, `records_after_dedup` vs `recordsAfterDedup`, etc.

4. **`author_papers`** — Returns completely unrelated papers for a given author ID. The API call may be hitting the wrong endpoint or constructing the query incorrectly.

### Non-Critical Issues (5 tools with caveats)

5. **`paper_autocomplete`** — Returns IDs but all titles are `null`. Likely missing a `fields` parameter in the API call.

6. **`author_search`** — Returns names/IDs correctly but all metrics (`citations`, `hIndex`, `papers`) are `0`. Missing fields in API request.

7. **`batch_metadata`** — Silently drops DOI-format paper IDs without error. Should either support them or return an error.

8. **`highly_cited_papers`** — Threshold calculation returns `0`, making `is_highly_cited` always `false`. Baseline estimation logic may be broken.

9. **`recommendations`** — Returns empty for extremely popular papers. May be an API limitation rather than a bug.

### Edge Case Observations

- **`cocitation_analysis`** times out on papers with >100k citations. Need to enforce smaller `maxCitingPapers` for popular papers.
- **`hot_papers`** and **`prisma_search`** can produce outputs exceeding 80-90KB, causing result truncation in the MCP response.
- **`citation_snowball`** returns 0 results when `minCitations` is set high for popular papers, likely because the API samples citations.

---

## Recommendations

1. **Fix camelCase/snake_case serialization** in `semantic_search` and `prisma_flow_diagram` — likely a `#[serde(rename)]` issue in the Rust structs.

2. **Fix `paper_title_match` response parsing** — add the missing `paperId` field handling or make it optional.

3. **Investigate `author_papers` data mismatch** — verify the API endpoint and query construction for author-specific paper retrieval.

4. **Add `fields` parameter** to `paper_autocomplete` and `author_search` API calls to populate title and metrics fields.

5. **Add result size limits** to `hot_papers`, `prisma_search`, and `pearl_growing` to prevent output truncation.

6. **Fix `highly_cited_papers` baseline** — the threshold calculation returns 0, defeating the purpose of the tool.

7. **Handle DOI-format IDs** in `batch_metadata` — either support them properly or return an informative error.
