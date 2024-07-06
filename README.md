CoLLector - crawls pages relevant to a given topic and extracts and returns structured node/edge data

uses `googler` [1] and `gptextract` (`simparse`, `callgpt`) [2]

[1] https://github.com/hversu/googler
[2] https://github.com/hversu/gptextract

## usage

`cargo run <topic> <entities>`

## example

`cargo run hversu country,language,culture,era,usage,english_translation`

## returns

(see `data/results.json`)
