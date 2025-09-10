success

```mermaid
flowchart TD
    N0["Pack<br><br>local:testsuite-online/git<br><br>testsuite-online/git"]
    N1["AptosFramework<br><br>git:github.com/aptos-labs/aptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/aptos-framework<br><br>cache/git/checkouts/github.com%2Faptos-labs%2Faptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/aptos-framework"]
    N2["AptosStdlib<br><br>git:github.com/aptos-labs/aptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/aptos-stdlib<br><br>cache/git/checkouts/github.com%2Faptos-labs%2Faptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/aptos-stdlib"]
    N3["MoveStdlib<br><br>git:github.com/aptos-labs/aptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/move-stdlib<br><br>cache/git/checkouts/github.com%2Faptos-labs%2Faptos-framework@fae5fee731d64e63e4028e27045792a053827dc5/move-stdlib"]
    N2 --> N3
    N1 --> N2
    N1 --> N3
    N0 --> N1
    N0 --> N3

```
