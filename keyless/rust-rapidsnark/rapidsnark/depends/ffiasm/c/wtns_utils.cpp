#include "wtns_utils.hpp"

namespace WtnsUtils {

Header::Header() {
}

Header::~Header() {
    mpz_clear(prime);
}

std::unique_ptr<Header> loadHeader(BinFileUtils::BinFile *f) {
    Header *h = new Header();
    f->startReadSection(1);

    h->n8 = f->readU32LE();
    mpz_init(h->prime);
    mpz_import(h->prime, h->n8, -1, 1, -1, 0, f->read(h->n8));

    h->nVars = f->readU32LE();

    f->endReadSection();

    return std::unique_ptr<Header>(h);
}

} // NAMESPACE