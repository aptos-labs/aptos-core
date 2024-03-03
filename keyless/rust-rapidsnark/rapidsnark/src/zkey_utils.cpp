#include <stdexcept>

#include "zkey_utils.hpp"

namespace ZKeyUtils {


Header::Header() {
}

Header::~Header() {
    mpz_clear(qPrime);
    mpz_clear(rPrime);
}


std::unique_ptr<Header> loadHeader(BinFileUtils::BinFile *f) {
    auto h = new Header();

    f->startReadSection(1);
    uint32_t protocol = f->readU32LE();
    if (protocol != 1) {
        throw std::invalid_argument( "zkey file is not groth16" );
    }
    f->endReadSection();

    f->startReadSection(2);

    h->n8q = f->readU32LE();
    mpz_init(h->qPrime);
    mpz_import(h->qPrime, h->n8q, -1, 1, -1, 0, f->read(h->n8q));

    h->n8r = f->readU32LE();
    mpz_init(h->rPrime);
    mpz_import(h->rPrime, h->n8r , -1, 1, -1, 0, f->read(h->n8r));

    h->nVars = f->readU32LE();
    h->nPublic = f->readU32LE();
    h->domainSize = f->readU32LE();

    h->vk_alpha1 = f->read(h->n8q*2);
    h->vk_beta1 = f->read(h->n8q*2);
    h->vk_beta2 = f->read(h->n8q*4);
    h->vk_gamma2 = f->read(h->n8q*4);
    h->vk_delta1 = f->read(h->n8q*2);
    h->vk_delta2 = f->read(h->n8q*4);
    f->endReadSection();

    h->nCoefs = f->getSectionSize(4) / (12 + h->n8r);

    return std::unique_ptr<Header>(h);
}

} // namespace

