colorscheme hybrid
imap <C-h> <Left>
imap <C-l> <Right>
inoremap " ""<LEFT>
inoremap ' ''<LEFT>
inoremap ( ()<LEFT>
inoremap [ []<LEFT>
inoremap { {}<LEFT>
autocmd InsertEnter,InsertLeave * set cursorline!
let g:neocomplcache_enable_at_startup = 1
let g:netrw_alto = 1
let g:netrw_altv = 1
let g:netrw_banner = 0
let g:netrw_liststyle = 3
let g:solarized_termcolors=256
let mapleader = "\<Space>"
nnoremap <C-n> :Ex<CR>
nnoremap <Leader>q :q!<cr>
nnoremap <leader>s :terminal<cr>
nnoremap <leader>w :wa<cr>
nnoremap j gj
nnoremap k gk
noremap <C-j> <esc>
noremap <leader>n :bn<CR>
noremap <leader>p :bp<CR>
noremap! <C-j> <esc>
runtime macros/matchit.vim
set smartindent
set autoread
set background=dark
set enc=utf8
set expandtab
set fenc=utf-8
set hidden
set hlsearch
set ignorecase
set incsearch
set laststatus=2
set nobackup
set noswapfile
set number
set shiftwidth=2
set showcmd
set showmatch
set tabstop=2
set virtualedit=onemore
set visualbell
set wildmenu
set wildmode=full
set wrapscan
syntax on
tnoremap <silent> jj <C-\><C-n>
nmap <Esc><Esc> :nohlsearch<CR><Esc>

" Plugin key-mappings.
imap <C-k>     <Plug>(neosnippet_expand_or_jump)
smap <C-k>     <Plug>(neosnippet_expand_or_jump)
xmap <C-k>     <Plug>(neosnippet_expand_target)

" SuperTab like snippets behavior.
"imap <expr><TAB>
" \ pumvisible() ? "\<C-n>" :
" \ neosnippet#expandable_or_jumpable() ?
" \    "\<Plug>(neosnippet_expand_or_jump)" : "\<TAB>"
smap <expr><TAB> neosnippet#expandable_or_jumpable() ?
            \ "\<Plug>(neosnippet_expand_or_jump)" : "\<TAB>"

function! Run ()
    :w
    :!gcc % -o %:r
    :!./%:r
endfunction

command! Gcc call Run()
nnoremap <leader>r :Gcc<CR>

" For conceal markers.
if has('conceal')
    set conceallevel=2 concealcursor=niv
endif

"set snippet file dir
let g:neosnippet#snippets_directory='~/.vim/bundle/neosnippet-snippets/snippets/,~/.vim/snippets'
set runtimepath+=~/.vim/bundle/neobundle.vim/
call neobundle#begin(expand('~/.vim/bundle/'))
NeoBundleFetch 'Shougo/neobundle.vim'
NeoBundle 'itchyny/lightline.vim'
NeoBundle 'Shougo/neocomplcache'
NeoBundle 'Shougo/neosnippet'
NeoBundle 'Shougo/neosnippet-snippets'
call neobundle#end()
filetype plugin indent on

