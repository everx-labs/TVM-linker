pragma solidity ^0.5.0;

// the interface of a remove contract
contract AnotherContract {
    function remoteMethod(uint64 value) pure public;
}

// this contract implement 'AnotherContract' interface
contract MyContract is AnotherContract {

    uint64 m_value;

    // A method to be called from another contract.
    // This method receive parameter 'value' from another contract and
    // and and save this value in persistent variable of this contract
    function remoteMethod(uint64 value) public {
        m_value = value;
        return;
    }

}
