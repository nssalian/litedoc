@profile litedoc

--- meta ---
from: data-extraction-agent
to: analysis-agent
message_id: msg_x7k9p2
in_reply_to: msg_a3b8c1
task: extract_and_forward
status: success
items_extracted: 47
---

# Extraction Complete

Extracted 47 records from the provided dataset.

## Summary

::table
| Metric | Value |
|--------|-------|
| Total rows | 1,247 |
| Valid rows | 1,203 |
| Extracted | 47 |
| Skipped | 1,156 |
| Errors | 44 |
::

## Extracted Data

::list
- 23 customer records matching criteria
- 18 transaction records from Q4
- 6 anomaly flags requiring review
::

## Errors Encountered

::callout type=warning title="Parse Failures"
44 rows failed validation:
- 31 missing required field `customer_id`
- 8 invalid date format
- 5 duplicate entries removed
::

## Next Steps

::list ordered
- Analysis agent should process extracted records
- Review anomaly flags manually
- Re-run extraction with corrected source data
::

---

Raw data follows in structured format for downstream processing.
