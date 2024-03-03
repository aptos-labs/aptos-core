#include <system_error>
#include <string>
#include <memory.h>
#include <stdexcept>

#include "binfile_utils.hpp"
#include "fileloader.hpp"

namespace BinFileUtils {

BinFile::BinFile(const void *fileData, size_t fileSize, std::string _type, uint32_t maxVersion) {

    size = fileSize;
    addr = malloc(size);
    memcpy(addr, fileData, size);

    type.assign((const char *)addr, 4);
    pos = 4;

    if (type != _type) {
        free(addr);
        throw new std::invalid_argument("Invalid file type. It should be " + _type + " and it is " + type);
    }

    version = readU32LE();
    if (version > maxVersion) {
        free(addr);
        throw new std::invalid_argument("Invalid version. It should be <=" + std::to_string(maxVersion) + " and it is " + std::to_string(version));
    }

    u_int32_t nSections = readU32LE();


    for (u_int32_t i=0; i<nSections; i++) {
        u_int32_t sType=readU32LE();
        u_int64_t sSize=readU64LE();

        if (sections.find(sType) == sections.end()) {
            sections.insert(std::make_pair(sType, std::vector<Section>()));
        }

        sections[sType].push_back(Section( (void *)((u_int64_t)addr + pos), sSize));

        pos += sSize;
    }

    pos = 0;
    readingSection = NULL;
}

BinFile::~BinFile() {
    free(addr);
}

void BinFile::startReadSection(u_int32_t sectionId, u_int32_t sectionPos) {

    if (sections.find(sectionId) == sections.end()) {
        throw new std::range_error("Section does not exist: " + std::to_string(sectionId));
    }

    if (sectionPos >= sections[sectionId].size()) {
        throw new std::range_error("Section pos too big. There are " + std::to_string(sections[sectionId].size()) + " and it's trying to access section: " + std::to_string(sectionPos));
    }

    if (readingSection != NULL) {
        throw new std::range_error("Already reading a section");
    }

    pos = (u_int64_t)(sections[sectionId][sectionPos].start) - (u_int64_t)addr;

    readingSection = &sections[sectionId][sectionPos];
}

void BinFile::endReadSection(bool check) {
    if (check) {
        if ((u_int64_t)addr + pos - (u_int64_t)(readingSection->start) != readingSection->size) {
            throw new std::range_error("Invalid section size");
        }
    }
    readingSection = NULL;
}

void *BinFile::getSectionData(u_int32_t sectionId, u_int32_t sectionPos) {

    if (sections.find(sectionId) == sections.end()) {
        throw new std::range_error("Section does not exist: " + std::to_string(sectionId));
    }

    if (sectionPos >= sections[sectionId].size()) {
        throw new std::range_error("Section pos too big. There are " + std::to_string(sections[sectionId].size()) + " and it's trying to access section: " + std::to_string(sectionPos));
    }

    return  sections[sectionId][sectionPos].start;
}

u_int64_t BinFile::getSectionSize(u_int32_t sectionId, u_int32_t sectionPos) {

    if (sections.find(sectionId) == sections.end()) {
        throw new std::range_error("Section does not exist: " + std::to_string(sectionId));
    }

    if (sectionPos >= sections[sectionId].size()) {
        throw new std::range_error("Section pos too big. There are " + std::to_string(sections[sectionId].size()) + " and it's trying to access section: " + std::to_string(sectionPos));
    }

    return  sections[sectionId][sectionPos].size;
}

u_int32_t BinFile::readU32LE() {
    u_int32_t res = *((u_int32_t *)((u_int64_t)addr + pos));
    pos += 4;
    return res;
}

u_int64_t BinFile::readU64LE() {
    u_int64_t res = *((u_int64_t *)((u_int64_t)addr + pos));
    pos += 8;
    return res;
}

void *BinFile::read(u_int64_t len) {
    void *res = (void *)((u_int64_t)addr + pos);
    pos += len;
    return res;
}

std::unique_ptr<BinFile> openExisting(std::string filename, std::string type, uint32_t maxVersion) {

    FileLoader fileLoader(filename);

    return std::unique_ptr<BinFile>(new BinFile(fileLoader.dataBuffer(), fileLoader.dataSize(), type, maxVersion));
}

} // Namespace

