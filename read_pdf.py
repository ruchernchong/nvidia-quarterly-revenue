import pdfplumber

from utils.replace_text import replace_text


def extract_data_from_pdf(pdf_path):
    data = {}

    try:
        with pdfplumber.open(pdf_path) as pdf:
            # Assuming the relevant data is on the first page
            page = pdf.pages[0]
            table = page.extract_table()

            if table:
                # Extract quarters from the first row, skipping the first column header
                quarters = table[0][1:]
                data["quarters"] = quarters[::-1]

                # Process the rest of the rows, skipping the first row (headers)
                for row in table[1:]:
                    if row:  # Ensure the row is not empty
                        key = replace_text(row[0].lower())
                        values = [
                            int(item.replace("$", "").replace(",", ""))
                            for item in row[1:]
                            if item
                        ]
                        data[key] = values[::-1]
            else:
                raise ValueError("No table found on the first page.")

    except FileNotFoundError:
        print(f"Error: The file '{pdf_path}' was not found.")
    except ValueError as ve:
        print(f"Error processing the PDF: {ve}")
    except Exception as e:
        print(f"An unexpected error occurred: {e}")

    return data
