-- Convert standalone mhchem math nodes for the chosen Pandoc writer.
-- Never rewrite ordinary text, code spans, code blocks, or the source note.
local function typst_string(s)
  s = s:gsub('\\', '\\\\'):gsub('"', '\\"')
       :gsub('\r', '\\r'):gsub('\n', '\\n'):gsub('\t', '\\t')
  return '"' .. s .. '"'
end

function Pandoc(doc)
  local used = false
  doc = doc:walk({ Math = function(el)
    if not el.text:find('\\ce%s*{') then return nil end
    used = true
    if FORMAT == 'latex' or FORMAT == 'beamer' then return nil end
    if FORMAT ~= 'typst' then
      error('Chemical math (\\ce) currently requires Typst PDF or LaTeX export.')
    end
    local formula = el.text:match('^%s*\\ce%s*(%b{})%s*$')
    if not formula then
      error('For Typst, put each \\ce{...} in its own math expression.')
    end
    local call = '#zettui-chem(' .. typst_string(formula:sub(2, -2)) .. ')'
    if el.mathtype == 'DisplayMath' then
      return pandoc.RawInline('typst', '$ ' .. call .. ' $')
    end
    return pandoc.RawInline('typst', '$' .. call .. '$')
  end })
  if used then
    local header
    if FORMAT == 'typst' then
      header = '#import "@preview/whalogen:0.3.0": ce as zettui-chem'
    else
      header = '\\usepackage{mhchem}'
    end
    local includes = doc.meta['header-includes']
    if includes == nil then
      includes = pandoc.MetaList({})
    elseif pandoc.utils.type(includes) ~= 'List' then
      includes = pandoc.MetaList({includes})
    end
    includes:insert(pandoc.MetaBlocks({pandoc.RawBlock(FORMAT, header)}))
    doc.meta['header-includes'] = includes
  end
  return doc
end
