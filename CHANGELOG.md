# Changelog

## Interface — rolagem, janela e modal - 2026-11-03
- Rolagem completa do catálogo nas abas Apps e Criação de Conteúdo: alturas fixas
  (content-height/height) trocadas por medida derivada do conteúdo real — cards
  abaixo do corte anterior (ferramentas e compatibilidade) voltam a ser acessados.
- Janela com tamanho de partida por preferred-width/height (1920×1040) e mínimos
  de 1024×648: abre dentro da tela e o redimensionamento não fica preso ao valor
  inicial fixo.
- Barra de título: duplo clique maximiza/restaura (TouchArea aninhada ao
  WindowMoveArea, mantendo o arrasto para mover a janela).
- Modal de detalhes: removido o × de fechar (resta o botão Fechar do canto
  inferior direito); clique fora do cartão fecha o modal; toques dentro do
  cartão e no escurecimento não mais atravessam para os elementos de fundo.

## Correção de compilação - 2026-11-02
- Removida largura percentual inválida da propriedade `content-width` do ScrollView.
- Removidos `min-width` e `min-height` conflitantes com as dimensões iniciais da janela.

## [0.1.0] - 2026-11-02

### Refinamentos visuais
- Expandida a ficha de detalhes com versão, desenvolvedor, origem e estado de atualização.
- Adicionadas ações contextuais de abrir, atualizar, remover, escolher fonte e instalar, com retorno visual que identifica as integrações ainda pendentes.
- Redesenhados os 14 ícones vetoriais dos aplicativos com melhor definição de silhueta, volume, contraste e paletas.
- Ajustados o raio dos cartões e o alinhamento interno entre ícone, nome, descrição e indicador de estado.
- Adicionada janela modal de detalhes ao selecionar cartões de aplicativos, com ícone, categoria, descrição e estado.
- Refinadas as superfícies de fundo e dos cartões com degradês por tema.
- Ajustados tamanhos tipográficos e estados de hover da navegação e dos controles superiores.
- Substituídos símbolos de status e setas por SVGs dedicados.
- Reduzida a altura de conteúdo excedente da rolagem para acompanhar o conteúdo real do catálogo.
- Substituídos os glifos provisórios do menu lateral e do botão de configurações por ícones SVG monocromáticos colorizados pelo tema.
- Ajustada a dimensão inicial da janela para 2048 px de largura, permitindo cinco cartões por linha na resolução de referência.
- Aplicado degradê azul-marinho ao conteúdo, elevação dos cartões e destaque visual para seleção e hover.
- Ações superiores de fontes e atualizações agora abrem suas respectivas telas provisórias.

### Adicionado
- Base da aplicação desktop em Rust com Slint 1.18.1.
- Janela principal com sidebar fixa, navegação e barra superior.
- Tela Apps com categorias de Launchers e Emuladores.
- Cartões reutilizáveis com ícones SVG locais, descrição, status e realce de seleção.
- Grade flexível que ajusta a quantidade de colunas conforme a largura disponível.
- ScrollView para rolagem vertical do conteúdo.
- Alternância entre tema escuro e tema claro por paleta compartilhada.
- Páginas provisórias para os demais itens de navegação.
- Documentação de execução e limitações atuais no README.

### Limitações
- Estados de instalação exibidos nos cartões são demonstrativos.
- Instalação, remoção, atualização, detecção de aplicativos e integração com gerenciadores de pacotes ainda não foram implementadas.
