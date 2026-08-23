# Third-party notices

## Excess vocabulary dataset

`data/excess-vocabulary.json` is a format-converted copy of
`results/excess_words.csv` from
[berenslab/chatgpt-excess-words](https://github.com/berenslab/chatgpt-excess-words),
revision `53db991afc251782106cd817a1c3fa47a4d41781`.

Paper:

> Kobak, D., González-Márquez, R., Horvát, E.-Á., and Lause, J. (2025).
> “Delving into LLM-assisted writing in biomedical publications through excess
> vocabulary.” *Science Advances*, 11(27), eadt3813.

The source repository annotates 900 observed excess words by content/style role,
including a small number of mixed or other labels. Prose Lint retains every
record for provenance but activates only the 407 entries labelled exactly
`style`. These observations come from biomedical abstracts and are
low-confidence, domain-specific review signals.

The source dataset is distributed under the MIT License:

```text
MIT License

Copyright (c) 2024 Dmitry Kobak, Rita González-Márquez

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Pattern references

The curated rule catalogue was written for this project. Its categories and
examples were informed by these public references:

- [Wikipedia: Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing)
- [blader/humanizer](https://github.com/blader/humanizer)
- [conorbronsdon/avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)
- [tbhb/vale-ai-tells](https://github.com/tbhb/vale-ai-tells)
- [seyedehsanhadi/sloptrim](https://github.com/seyedehsanhadi/sloptrim)

No source code from those projects is included.
