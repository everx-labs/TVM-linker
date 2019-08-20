pragma solidity ^0.5.0;

contract TestArray {

    function at32(uint8 idx, uint32[] memory arr) public pure returns (uint32) {
        return arr[idx];
    }

    function at256(uint8 idx, uint256[] memory arr) public pure returns (uint256) {
        return arr[idx];
    }

    function atAt32(uint32 idx, uint32[] memory arr, uint32 idy) public pure returns (uint32) {
        return arr[idx + idy];
    }

    function atAt256(uint32 idx, uint256[] memory arr, uint32 idy) public pure returns (uint256) {
        return arr[idx + idy];
    }
}
