.PHONY: all clean paper zip

PAPER = aegis-paper
LATEXMK = latexmk
LATEXMKFLAGS = -pdf -pdflatex="pdflatex -interaction=nonstopmode" -use-make

all: paper

paper:
	$(LATEXMK) $(LATEXMKFLAGS) $(PAPER)
	@echo "Paper built: $(PAPER).pdf"

clean:
	$(LATEXMK) -C $(PAPER)
	rm -f *.bbl *.blg *.run.xml *.bcf *.synctex.gz
	rm -rf figs/

# Springer LNCS submission package
zip: paper
	@mkdir -p submission-package
	@cp $(PAPER).pdf submission-package/
	@cp $(PAPER).tex submission-package/
	@cp llncs.cls splncs04.bst submission-package/
	@cp -r figs submission-package/ 2>/dev/null || true
	@cp ../LICENSE submission-package/
	@cp README-submission.md submission-package/README.md
	@echo "Creating $(PAPER)-springer-submission.zip..."
	@cd submission-package && zip -r ../$(PAPER)-springer-submission.zip .
	@rm -rf submission-package
	@echo "Created $(PAPER)-springer-submission.zip"
	@du -sh $(PAPER)-springer-submission.zip
