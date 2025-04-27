import os
import glob
import argparse

def merge_headers(header_dir, output_header):
    # HEADER_DIR içindeki tüm .h dosyalarını listele
    header_files = glob.glob(os.path.join(header_dir, '*.h'))

    # Birleştirilmiş başlık dosyasını oluştur ve yazma modunda aç
    with open(output_header, 'w') as outfile:
        # Dosyanın başına açıklama satırları ekle
        outfile.write('// Otomatik oluşturulmuş birleştirilmiş başlık dosyası\n')
        outfile.write(f'// Kaynak dizin: {header_dir}\n\n')
        outfile.write('#pragma once\n\n')

        # Her bir başlık dosyasını sırayla işle
        for header_file in header_files:
            # Dosya adını birleştirilmiş dosyaya ekle
            outfile.write(f'// {header_file}\n')
            # Başlık dosyasını okuyup içeriğini birleştirilmiş dosyaya yaz
            with open(header_file, 'r') as infile:
                outfile.write(infile.read())
                outfile.write('\n\n')  # Dosyalar arasında boşluk bırak

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Başlık dosyalarını birleştirir.')
    parser.add_argument('--header_dir', type=str, required=True, help='Başlık dosyalarının bulunduğu dizin')
    parser.add_argument('--output_header', type=str, required=True, help='Oluşturulacak birleştirilmiş başlık dosyası')
    args = parser.parse_args()
    merge_headers(args.header_dir, args.output_header)

