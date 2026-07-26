.PHONY: all paper clean

LATEXMK = latexmk
LATEXMKFLAGS = -pdf -pdflatex="pdflatex -interaction=nonstopmode" -use-make

all: paper

paper:
	$(LATEXMK) $(LATEXMKFLAGS) aegis-paper.tex

clean:
	$(LATEXMK) -C
	rm -f *.bbl *.blg *.run.xml *.bcf *.synctex.gz

zip:
	@mkdir -p submission
	@cp aegis-paper.tex llncs.cls submission/
	@cp -r figs submission/ 2>/dev/null || true
	@cd submission && zip -r ../aegis-springer-submission.zip .
	@rm -rf submission
	@echo "Created aegis-springer-submission.zip"
