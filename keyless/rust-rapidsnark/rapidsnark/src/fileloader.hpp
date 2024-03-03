#ifndef FILELOADER_HPP
#define FILELOADER_HPP

#include <cstddef>
#include <string>

namespace BinFileUtils {

class FileLoader
{
public:
    FileLoader(const std::string& fileName);
    ~FileLoader();

    void*  dataBuffer() { return addr; }
    size_t dataSize() const { return size; }

private:
    void*   addr;
    size_t  size;
    int     fd;
};

}

#endif // FILELOADER_HPP
